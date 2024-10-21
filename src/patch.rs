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

use super::lv2::{Plugin, Plugins};
use super::session::Processor;
use std::collections::hash_map::{self, HashMap};
use std::fmt::{self, Display};
use std::ops::Range;

#[derive(Debug)]
struct Replacement {
    pub location: Range<usize>,
    pub value: u32,
}

#[derive(Debug)]
pub struct PatchedSession<'a> {
    xml: &'a str,
    replacements: Vec<Replacement>,
}

macro_rules! debug_eprintln {
    ($($tt:tt)*) => {
        if cfg!(debug_assertions) {
            eprintln!($($tt)*);
        }
    };
}

impl Display for PatchedSession<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pos = 0;
        for r in self.replacements.iter() {
            if r.location.start < pos {
                debug_eprintln!(
                    "warning: overlapping/out-of-order replacement: \
                     {}..{} -> {} (currently at {pos})",
                    r.location.start,
                    r.location.end,
                    r.value,
                );
                continue;
            }
            write!(f, "{}{}", &self.xml[pos..r.location.start], r.value)?;
            pos = r.location.end;
        }
        write!(f, "{}", &self.xml[pos..])
    }
}

#[derive(Debug)]
pub enum Error {
    Xml(roxmltree::Error),
    Lv2(super::lv2::Error),
}

impl From<roxmltree::Error> for Error {
    fn from(e: roxmltree::Error) -> Self {
        Self::Xml(e)
    }
}

impl From<super::lv2::Error> for Error {
    fn from(e: super::lv2::Error) -> Self {
        Self::Lv2(e)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Xml(e) => {
                write!(f, "could not parse session file: {e}")
            }
            Self::Lv2(e) => {
                write!(f, "could not retrieve lv2 metadata: {e}")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct PortId<'a> {
    pub uri: &'a str,
    pub symbol: &'a str,
}

#[derive(Debug, Default)]
struct PortMap<'a> {
    count: HashMap<&'a str, u32>,
    index: HashMap<PortId<'a>, u32>,
}

impl<'a> PortMap<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn index(&mut self, plugin: &mut Plugin, id: PortId<'a>) -> u32 {
        let vacant = match self.index.entry(id) {
            hash_map::Entry::Occupied(ent) => return *ent.get(),
            hash_map::Entry::Vacant(ent) => ent,
        };
        if let Some(i) = plugin.port_index(id.symbol) {
            return *vacant.insert(i);
        }
        eprintln!(
            "warning: could not find port \"{}\" in {}",
            id.symbol.escape_default(),
            id.uri,
        );
        let count =
            self.count.entry(id.uri).or_insert_with(|| plugin.num_ports());
        *vacant.insert(std::mem::replace(count, *count + 1))
    }
}

struct Patcher<'a, 'xml> {
    root: roxmltree::Node<'a, 'xml>,
    plugins: Plugins,
    ports: PortMap<'a>,
    replacements: Vec<Replacement>,
}

impl<'a, 'xml> Patcher<'a, 'xml> {
    fn handle_processor(&mut self, processor: Processor<'a>) {
        let uri = processor.uri();
        let Some(mut plugin) = self.plugins.get(uri) else {
            eprintln!("warning: could not find plugin: {uri}");
            return;
        };
        for parameter in processor.parameters() {
            let index = self.ports.index(&mut plugin, PortId {
                uri,
                symbol: parameter.symbol,
            });
            if index == parameter.old_index {
                continue;
            }
            self.replacements.push(Replacement {
                location: parameter.location,
                value: index,
            })
        }
    }

    fn populate_replacements(&mut self) -> Result<(), Error> {
        let mut next = Some(self.root);
        while let Some(node) = next {
            next = None;
            if node.has_tag_name("Processor") {
                if let Some(p) = Processor::parse(node) {
                    self.handle_processor(p);
                }
            } else {
                next = node.first_child();
            }
            next = next.or_else(|| {
                node.ancestors().filter_map(|a| a.next_sibling()).next()
            });
        }
        Ok(())
    }

    fn run(mut self) -> Result<PatchedSession<'xml>, Error> {
        self.populate_replacements()?;
        self.replacements.sort_unstable_by_key(|r| r.location.start);
        Ok(PatchedSession {
            xml: self.root.document().input_text(),
            replacements: self.replacements,
        })
    }
}

pub fn patch(xml: &str) -> Result<PatchedSession<'_>, Error> {
    Patcher {
        root: roxmltree::Document::parse(xml)?.root(),
        plugins: Plugins::new()?,
        ports: PortMap::new(),
        replacements: Vec::new(),
    }
    .run()
}
