// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use dbus::arg::IterAppend;
use dbus_tree::{MTSync, MethodErr, PropInfo};

use crate::{
    dbus_api::{
        api::shared::{self, get_manager_property},
        types::TData,
    },
    engine::Engine,
    stratis::VERSION,
};

pub fn get_version<E>(
    i: &mut IterAppend<'_>,
    _: &PropInfo<'_, MTSync<TData<E>>, TData<E>>,
) -> Result<(), MethodErr>
where
    E: Engine,
{
    i.append(VERSION);
    Ok(())
}

pub fn get_locked_pools<E>(
    i: &mut IterAppend<'_>,
    p: &PropInfo<'_, MTSync<TData<E>>, TData<E>>,
) -> Result<(), MethodErr>
where
    E: Engine,
{
    get_manager_property(i, p, |e| Ok(shared::locked_pools_prop(e)))
}
