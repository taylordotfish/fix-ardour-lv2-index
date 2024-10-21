/*
 * Copyright (C) 2024 taylor.fish <contact@taylor.fish>
 *
 * This file is part of fix-ardour-lv2-index.
 *
 * fix-ardour-lv2-index is free software: you can redistribute it and/or
 * modify it under the terms of the GNU General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * fix-ardour-lv2-index is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
 * General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with fix-ardour-lv2-index. If not, see <https://www.gnu.org/licenses/>.
 */

#![allow(clippy::undocumented_unsafe_blocks)]

use std::ffi::CString;
use std::fmt::{self, Display};
use std::marker::PhantomData;
use std::ptr::NonNull;

mod lilv {
    use std::ffi::{c_char, c_void};
    use std::marker::{PhantomData, PhantomPinned};

    type Phantom = PhantomData<(*mut u8, PhantomPinned)>;
    #[repr(C)]
    pub struct LilvNode([u8; 0], Phantom);
    #[repr(C)]
    pub struct LilvPlugin([u8; 0], Phantom);
    pub type LilvPlugins = c_void;
    #[repr(C)]
    pub struct LilvPort([u8; 0], Phantom);
    #[repr(C)]
    pub struct LilvWorld([u8; 0], Phantom);

    #[link(name = "lilv-0")]
    extern "C" {
        pub fn lilv_new_string(
            world: *mut LilvWorld,
            r#str: *const c_char,
        ) -> *mut LilvNode;
        pub fn lilv_new_uri(
            world: *mut LilvWorld,
            uri: *const c_char,
        ) -> *mut LilvNode;
        pub fn lilv_node_free(val: *mut LilvNode);
        pub fn lilv_plugin_get_num_ports(plugin: *const LilvPlugin) -> u32;
        pub fn lilv_plugin_get_port_by_symbol(
            plugin: *const LilvPlugin,
            symbol: *const LilvNode,
        ) -> *const LilvPort;
        pub fn lilv_plugins_get_by_uri(
            plugins: *const LilvPlugins,
            uri: *const LilvNode,
        ) -> *const LilvPlugin;
        pub fn lilv_port_get_index(
            plugin: *const LilvPlugin,
            port: *const LilvPort,
        ) -> u32;
        pub fn lilv_world_free(world: *mut LilvWorld);
        pub fn lilv_world_get_all_plugins(
            world: *const LilvWorld,
        ) -> *const LilvPlugins;
        pub fn lilv_world_load_all(world: *mut LilvWorld);
        pub fn lilv_world_new() -> *mut LilvWorld;
    }
}

use lilv as lv;

#[derive(Debug)]
pub enum Error {
    LilvWorldNew,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LilvWorldNew => write!(f, "lilv_world_new failed"),
        }
    }
}

pub struct Plugins {
    world: NonNull<lv::LilvWorld>,
    plugins: NonNull<lv::LilvPlugins>,
}

impl Plugins {
    pub fn new() -> Result<Self, Error> {
        let world = NonNull::new(unsafe { lv::lilv_world_new() })
            .ok_or(Error::LilvWorldNew)?;
        unsafe {
            lv::lilv_world_load_all(world.as_ptr());
        }
        let plugins = NonNull::new(unsafe {
            lv::lilv_world_get_all_plugins(world.as_ptr())
        } as _)
        .expect("lilv_world_get_all_plugins failed");
        Ok(Self {
            plugins,
            world,
        })
    }

    pub fn get(&mut self, uri: &str) -> Option<Plugin<'_>> {
        let Ok(uri) = CString::new(uri) else {
            eprintln!("warning: \\0 in uri: \"{}\"", uri.escape_default());
            return None;
        };
        let node = NonNull::new(unsafe {
            lv::lilv_new_uri(self.world.as_ptr(), uri.as_ptr())
        })
        .expect("lilv_new_uri failed");
        let plugin = NonNull::new(unsafe {
            lv::lilv_plugins_get_by_uri(self.plugins.as_ptr(), node.as_ptr())
        } as _);
        unsafe {
            lv::lilv_node_free(node.as_ptr());
        }
        plugin.map(|p| Plugin {
            world: self.world,
            plugin: p,
            _phantom: PhantomData,
        })
    }
}

impl Drop for Plugins {
    fn drop(&mut self) {
        unsafe {
            lv::lilv_world_free(self.world.as_ptr());
        }
    }
}

pub struct Plugin<'a> {
    world: NonNull<lv::LilvWorld>,
    plugin: NonNull<lv::LilvPlugin>,
    _phantom: PhantomData<&'a mut lv::LilvWorld>,
}

impl Plugin<'_> {
    pub fn num_ports(&self) -> u32 {
        unsafe { lv::lilv_plugin_get_num_ports(self.plugin.as_ptr()) }
    }

    pub fn port_index(&mut self, symbol: &str) -> Option<u32> {
        let Ok(symbol) = CString::new(symbol) else {
            eprintln!(
                "warning: \\0 in symbol: \"{}\"",
                symbol.escape_default(),
            );
            return None;
        };
        let node = NonNull::new(unsafe {
            lv::lilv_new_string(self.world.as_ptr(), symbol.as_ptr())
        })
        .expect("lilv_new_string failed");
        let port = NonNull::new(unsafe {
            lv::lilv_plugin_get_port_by_symbol(
                self.plugin.as_ptr(),
                node.as_ptr(),
            )
        } as _);
        unsafe {
            lv::lilv_node_free(node.as_ptr());
        }
        let port = port?;
        Some(unsafe {
            lv::lilv_port_get_index(self.plugin.as_ptr(), port.as_ptr())
        })
    }
}
