// Copyright 2019 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod objdump;

use std::ffi::{OsStr,OsString};
use std::path::Path;

/// SizeGraph is a directed graph of symbols in an ELF binary. It contains the
/// size of each symbol, as well as the forward and reverse dependencies of that
/// symbol (for graph traversal). It is constructed using the load() method,
/// which loads a binary from the filesystem.
pub struct SizeGraph {
    name_to_idx: std::collections::HashMap::<Vec<u8>, usize>,
    symbols: Vec<SymbolData>,
}

impl SizeGraph {
    // Reads the provided ELF executable and returns the size graph
    // corresponding to that executable. objdump is the name of the objdump
    // binary to use. find_objdump() is provided to supply this flag in binaries
    // that do not have their own argument parsing logic.
    pub fn load<S: AsRef<OsStr>, P: AsRef<Path>>(objdump: S, path: P)
        -> Result<SizeGraph, LoadError>
    {
        use rustc_demangle::demangle;

        // We can run objdump asynchronously (spawn it as a process then wait on
        // it later), but elf is synchronous. We exploit a bit of parallelism
        // by letting objdump run while we use the elf crate.
        let objdump_stdout = std::process::Command::new(objdump)
            .arg("-d").arg(path.as_ref()).stdout(std::process::Stdio::piped())
            .spawn()?.stdout.expect("stdout pipe not found");

        let mut symbols = Vec::new();

        // Maps mangled names to their indexes in symbols. Keyed with [u8]
        // rather than a str so that we don't need to perform UTF-8 validation
        // on objdump's output.
        let mut name_to_idx = std::collections::HashMap::new();

        // Use the `elf` crate to get the sizes of symbols. We demangle the
        // names as we find them.
        let elf_file = elf::File::open_path(path)?;
        for section in &elf_file.sections {
            for elf_symbol in elf_file.get_symbols(&section)? {
                let demangled_name = demangle(&elf_symbol.name).to_string();
                name_to_idx.insert(elf_symbol.name.into_bytes(), symbols.len());
                symbols.push(SymbolData {
                    name: demangled_name,
                    size: elf_symbol.size as usize,
                    deps: Vec::new(),
                    rev_deps: Vec::new(),
                });
            }
        }

        // Process objdump's output to generate the dependency tree.
        let current_symbol = std::cell::Cell::new(None);
        objdump::parse(objdump_stdout,
            |symbol| {
                current_symbol.set(name_to_idx.get(symbol));
                if current_symbol.get().is_none() {
                    eprintln!("objdump referenced unknown symbol {}",
                              String::from_utf8_lossy(symbol));
                }
            },
            |symbol| {
                let current_symbol = match current_symbol.get() {
                    None => return,
                    Some(&sym) => sym,
                };
                let target_symbol = match name_to_idx.get(symbol) {
                    None => {
                        eprintln!("objdump referenced unknown symbol {}",
                                  String::from_utf8_lossy(symbol));
                        return;
                    },
                    Some(&sym) => sym,
                };
                symbols[current_symbol].deps.push(target_symbol);
                symbols[target_symbol].rev_deps.push(current_symbol);
            }
        )?;

        Ok(SizeGraph { name_to_idx, symbols })
    }

    // Retrieve a symbol by demangled name.
    pub fn get(&self, name: &str) -> Option<Symbol> {
        Some(Symbol::new(&self, *self.name_to_idx.get(name.as_bytes())?))
    }

    // Return an iterator that iterates through all symbols in this graph.
    pub fn iter(&self) -> SymbolIter {
        SymbolIter {
            graph: self,
            current: 0,
        }
    }

    // Returns the number of symbols in this graph.
    pub fn len(&self) -> usize {
        self.symbols.len()
    }
}

pub struct Symbol<'g> {
    graph: &'g SizeGraph,
    index: usize,
}

impl<'g> Symbol<'g> {
    pub fn new(graph: &'g SizeGraph, index: usize) -> Symbol<'g> {
        Symbol { graph, index }
    }

    pub fn name(&self) -> &str {
        &self.graph.symbols[self.index].name
    }

    pub fn size(&self) -> usize {
        self.graph.symbols[self.index].size
    }

    pub fn deps(&self) -> Vec<Symbol> {
        self.graph.symbols[self.index].deps.iter()
            .map(|&i| Symbol::new(self.graph, i)).collect()
    }

    pub fn reverse_deps(&self) -> Vec<Symbol> {
        self.graph.symbols[self.index].rev_deps.iter()
            .map(|&i| Symbol::new(self.graph, i)).collect()
    }
}

pub enum LoadError {
    ProcessError(std::io::Error),  // Launching objdump failed
    ElfError(elf::ParseError),  // The elf crate failed to parse the binary
}

/// Scans through the command line arguments, searching for an
/// "--objdump OBJDUMP" argument pair. Provided as a convenience for tools that
/// don't have their own command line argument parser. If no --objdump flag
/// exists, defaults to "objdump".
pub fn find_objdump() -> Result<OsString, ArgError> {
    let mut args = std::env::args_os();
    while let Some(arg) = args.next() {
        if arg != "--objdump" { continue; }
        match args.next() {
            None => return Err(ArgError::FlagWithoutValue),
            Some(objdump) => return Ok(objdump),
        }
    }
    Ok("objdump".into())
}

pub enum ArgError {
    // --objdump was provided as the last argument, i.e. there is no associated
    // value.
    FlagWithoutValue,
}

/// Iterator to scan through all symbols in the size graph.
pub struct SymbolIter<'g> {
    graph: &'g SizeGraph,
    current: usize,
}

impl<'g> Iterator for SymbolIter<'g> {
    type Item = Symbol<'g>;

    fn next(&mut self) -> Option<Symbol<'g>> {
        if self.current >= self.graph.len() { return None; }
        let out = Symbol::new(self.graph, self.current);
        self.current += 1;
        Some(out)
    }
}

// -----------------------------------------------------------------------------
// Implementation details below
// -----------------------------------------------------------------------------

impl std::convert::From<std::io::Error> for LoadError {
    fn from(io_error: std::io::Error) -> LoadError {
        LoadError::ProcessError(io_error)
    }
}

impl std::convert::From<elf::ParseError> for LoadError {
    fn from(parse_error: elf::ParseError) -> LoadError {
        LoadError::ElfError(parse_error)
    }
}

// Symbol data used inside SizeGraph -- does not hold the references needed to
// implement Symbol's interface.
struct SymbolData {
    name: String,
    size: usize,
    deps: Vec<usize>,  // Indexes into the symbols vector.
    rev_deps: Vec<usize>,
}
