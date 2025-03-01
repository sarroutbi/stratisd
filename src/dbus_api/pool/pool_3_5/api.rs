// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use dbus_tree::{Factory, MTSync, Method};

use crate::{
    dbus_api::{pool::pool_3_5::methods::init_cache, types::TData},
    engine::Engine,
};

pub fn init_cache_method<E>(
    f: &Factory<MTSync<TData<E>>, TData<E>>,
) -> Method<MTSync<TData<E>>, TData<E>>
where
    E: 'static + Engine,
{
    f.method("InitCache", (), init_cache)
        .in_arg(("devices", "as"))
        // b: Indicates if any cache devices were added
        // ao: Array of object paths of created cache devices
        //
        // Rust representation: (bool, Vec<dbus::path>)
        .out_arg(("results", "(bao)"))
        .out_arg(("return_code", "q"))
        .out_arg(("return_string", "s"))
}
