// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::{os::unix::io::RawFd, path::Path, sync::Arc};

use tokio::task::block_in_place;

use crate::{
    engine::{
        BlockDevTier, CreateAction, EncryptionInfo, Engine, EngineAction, Name, Pool,
        PoolIdentifier, PoolUuid, RenameAction, UnlockMethod,
    },
    jsonrpc::{
        interface::PoolListType,
        server::key::{key_get_desc, key_set},
    },
    stratis::{StratisError, StratisResult},
};

// stratis-min pool start
pub async fn pool_start<E>(
    engine: Arc<E>,
    id: PoolIdentifier<PoolUuid>,
    unlock_method: Option<UnlockMethod>,
    prompt: Option<RawFd>,
) -> StratisResult<bool>
where
    E: Engine,
{
    if let (Some(fd), Some(kd)) = (prompt, key_get_desc(engine.clone(), id.clone()).await?) {
        key_set(engine.clone(), &kd, fd).await?;
    }

    Ok(engine.start_pool(id, unlock_method).await?.is_changed())
}

// stratis-min pool stop
pub async fn pool_stop<E>(engine: Arc<E>, id: PoolIdentifier<PoolUuid>) -> StratisResult<bool>
where
    E: Engine,
{
    let pool_uuid = match id {
        PoolIdentifier::Uuid(u) => u,
        PoolIdentifier::Name(n) => {
            engine
                .pools()
                .await
                .get_by_name(&n)
                .ok_or_else(|| StratisError::Msg(format!("Pool with name {n} not found")))?
                .0
        }
    };
    Ok(engine.stop_pool(pool_uuid).await?.is_changed())
}

// stratis-min pool create
pub async fn pool_create<E>(
    engine: Arc<E>,
    name: &str,
    blockdev_paths: &[&Path],
    enc_info: Option<&EncryptionInfo>,
) -> StratisResult<bool>
where
    E: Engine,
{
    Ok(
        match engine.create_pool(name, blockdev_paths, enc_info).await? {
            CreateAction::Created(_) => true,
            CreateAction::Identity => false,
        },
    )
}

// stratis-min pool destroy
pub async fn pool_destroy<E>(engine: Arc<E>, name: &str) -> StratisResult<bool>
where
    E: Engine,
{
    let uuid = engine
        .get_pool(PoolIdentifier::Name(Name::new(name.to_owned())))
        .await
        .map(|g| g.as_tuple().1)
        .ok_or_else(|| StratisError::Msg(format!("No pool named {name} found")))?;
    Ok(engine.destroy_pool(uuid).await?.is_changed())
}

// stratis-min pool init-cache
pub async fn pool_init_cache<E>(engine: Arc<E>, name: &str, paths: &[&Path]) -> StratisResult<bool>
where
    E: Engine,
{
    let mut guard = engine
        .get_mut_pool(PoolIdentifier::Name(Name::new(name.to_owned())))
        .await
        .ok_or_else(|| StratisError::Msg(format!("No pool named {name} found")))?;
    let (_, uuid, pool) = guard.as_mut_tuple();
    block_in_place(|| Ok(pool.init_cache(uuid, name, paths, true)?.is_changed()))
}

// stratis-min pool rename
pub async fn pool_rename<E>(
    engine: Arc<E>,
    current_name: &str,
    new_name: &str,
) -> StratisResult<bool>
where
    E: Engine,
{
    let uuid = engine
        .get_pool(PoolIdentifier::Name(Name::new(current_name.to_owned())))
        .await
        .map(|g| g.as_tuple().1)
        .ok_or_else(|| StratisError::Msg(format!("No pool named {current_name} found")))?;
    Ok(match engine.rename_pool(uuid, new_name).await? {
        RenameAction::Identity => false,
        RenameAction::Renamed(_) => true,
        RenameAction::NoSource => unreachable!(),
    })
}

// stratis-min pool add-data
pub async fn pool_add_data<E>(
    engine: Arc<E>,
    name: &str,
    blockdevs: &[&Path],
) -> StratisResult<bool>
where
    E: Engine,
{
    add_blockdevs(engine, name, blockdevs, BlockDevTier::Data).await
}

// stratis-min pool add-cache
pub async fn pool_add_cache<E>(
    engine: Arc<E>,
    name: &str,
    blockdevs: &[&Path],
) -> StratisResult<bool>
where
    E: Engine,
{
    add_blockdevs(engine, name, blockdevs, BlockDevTier::Cache).await
}

async fn add_blockdevs<E>(
    engine: Arc<E>,
    name: &str,
    blockdevs: &[&Path],
    tier: BlockDevTier,
) -> StratisResult<bool>
where
    E: Engine,
{
    let mut guard = engine
        .get_mut_pool(PoolIdentifier::Name(Name::new(name.to_owned())))
        .await
        .ok_or_else(|| StratisError::Msg(format!("No pool named {name} found")))?;
    let (_, uuid, pool) = guard.as_mut_tuple();
    block_in_place(|| {
        Ok(pool
            .add_blockdevs(uuid, name, blockdevs, tier)?
            .is_changed())
    })
}

// stratis-min pool [list]
pub async fn pool_list<E>(engine: Arc<E>) -> PoolListType
where
    E: Engine,
{
    let guard = engine.pools().await;
    guard
        .iter()
        .map(|(n, u, p)| {
            (
                n.to_string(),
                (
                    *p.total_physical_size().bytes(),
                    p.total_physical_used().map(|u| *u.bytes()),
                ),
                (p.has_cache(), p.is_encrypted()),
                u,
            )
        })
        .fold(
            (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
            |(mut name_vec, mut size_vec, mut pool_props_vec, mut uuid_vec), (n, s, p, u)| {
                name_vec.push(n);
                size_vec.push(s);
                pool_props_vec.push(p);
                uuid_vec.push(*u);
                (name_vec, size_vec, pool_props_vec, uuid_vec)
            },
        )
}

// stratis-min pool is-encrypted
pub async fn pool_is_encrypted<E>(
    engine: Arc<E>,
    id: PoolIdentifier<PoolUuid>,
) -> StratisResult<bool>
where
    E: Engine,
{
    let locked = engine.locked_pools().await;
    let guard = engine.get_pool(id.clone()).await;
    if let Some((_, _, pool)) = guard.as_ref().map(|guard| guard.as_tuple()) {
        Ok(pool.is_encrypted())
    } else if locked
        .locked
        .get(match id {
            PoolIdentifier::Uuid(ref u) => u,
            PoolIdentifier::Name(ref n) => locked
                .name_to_uuid
                .get(n)
                .ok_or_else(|| StratisError::Msg(format!("Could not find pool with name {n}")))?,
        })
        .is_some()
    {
        Ok(true)
    } else {
        Err(StratisError::Msg(format!("Pool with {id} not found")))
    }
}

// stratis-min pool is-stopped
pub async fn pool_is_stopped<E>(engine: Arc<E>, id: PoolIdentifier<PoolUuid>) -> StratisResult<bool>
where
    E: Engine,
{
    let stopped = engine.stopped_pools().await;
    if engine.get_pool(id.clone()).await.is_some() {
        Ok(false)
    } else if stopped
        .stopped
        .get(match id {
            PoolIdentifier::Uuid(ref u) => u,
            PoolIdentifier::Name(ref n) => stopped
                .name_to_uuid
                .get(n)
                .ok_or_else(|| StratisError::Msg(format!("Could not find pool with name {n}")))?,
        })
        .is_some()
    {
        Ok(true)
    } else {
        Err(StratisError::Msg(format!("Pool with {id} not found")))
    }
}

// stratis-min pool is-bound
pub async fn pool_is_bound<E>(engine: Arc<E>, id: PoolIdentifier<PoolUuid>) -> StratisResult<bool>
where
    E: Engine,
{
    let locked = engine.locked_pools().await;
    let guard = engine.get_pool(id.clone()).await;
    if let Some((_, _, pool)) = guard.as_ref().map(|guard| guard.as_tuple()) {
        Ok(match pool.encryption_info() {
            Some(ei) => ei.clevis_info()?.is_some(),
            None => false,
        })
    } else if let Some(info) = locked.locked.get(match id {
        PoolIdentifier::Uuid(ref u) => u,
        PoolIdentifier::Name(ref n) => locked
            .name_to_uuid
            .get(n)
            .ok_or_else(|| StratisError::Msg(format!("Could not find pool with name {n}")))?,
    }) {
        Ok(info.info.clevis_info()?.is_some())
    } else {
        Err(StratisError::Msg(format!("Pool with UUID {id} not found")))
    }
}

// stratis-min pool has-passphrase
pub async fn pool_has_passphrase<E>(
    engine: Arc<E>,
    id: PoolIdentifier<PoolUuid>,
) -> StratisResult<bool>
where
    E: Engine,
{
    let locked = engine.locked_pools().await;
    let guard = engine.get_pool(id.clone()).await;
    if let Some((_, _, pool)) = guard.as_ref().map(|guard| guard.as_tuple()) {
        Ok(match pool.encryption_info() {
            Some(ei) => ei.key_description()?.is_some(),
            None => false,
        })
    } else if let Some(info) = locked.locked.get(match id {
        PoolIdentifier::Uuid(ref u) => u,
        PoolIdentifier::Name(ref n) => locked
            .name_to_uuid
            .get(n)
            .ok_or_else(|| StratisError::Msg(format!("Could not find pool with name {n}")))?,
    }) {
        Ok(info.info.key_description()?.is_some())
    } else {
        Err(StratisError::Msg(format!("Pool with {id} not found")))
    }
}

// stratis-min pool clevis-pin
pub async fn pool_clevis_pin<E>(
    engine: Arc<E>,
    id: PoolIdentifier<PoolUuid>,
) -> StratisResult<Option<String>>
where
    E: Engine,
{
    let locked = engine.locked_pools().await;
    let guard = engine.get_pool(id.clone()).await;
    if let Some((_, _, pool)) = guard.as_ref().map(|guard| guard.as_tuple()) {
        let encryption_info = match pool.encryption_info() {
            Some(ei) => EncryptionInfo::try_from(ei)?,
            None => return Ok(None),
        };
        Ok(encryption_info.clevis_info().map(|(pin, _)| pin.clone()))
    } else if let Some(info) = locked.locked.get(match id {
        PoolIdentifier::Uuid(ref u) => u,
        PoolIdentifier::Name(ref n) => locked
            .name_to_uuid
            .get(n)
            .ok_or_else(|| StratisError::Msg(format!("Could not find pool with name {n}")))?,
    }) {
        Ok(info.info.clevis_info()?.map(|(pin, _)| pin.clone()))
    } else {
        Err(StratisError::Msg(format!("Pool with {id} not found")))
    }
}
