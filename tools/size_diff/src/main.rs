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

/// size_diff compares two ELF files to determine why they differ in size. It is
/// intended to be used to evaluate the effect of code changes on the size of a
/// binary.

/// Contains interesting size data for an ELF file.
struct SizeData {
    /// A map from a symbol's demangled name to its size.
    pub name_to_size: std::collections::HashMap<String, isize>,

    /// Size of .data, if present.
    pub data_size: isize,

    /// Size of .rodata, if present.
    pub rodata_size: isize,
}

fn read_elf(file: &str) -> SizeData {
    let elf_file = elf::File::open_path(file)
        .expect(&format!("Unable to load file {}", file));

    let mut name_to_size = std::collections::HashMap::new();
    let mut data_size = 0;
    let mut rodata_size = 0;

    for section in &elf_file.sections {
        // Record the sizes of .data and .rodata.
        match section.shdr.name.as_ref() {
            ".data" => data_size += section.shdr.size as isize,
            ".rodata" => rodata_size += section.shdr.size as isize,
            _ => {},
        }

        let symbols = elf_file.get_symbols(&section)
            .expect(&format!("Unable to read symbols from section {}", section));
        for symbol in symbols {
            use rustc_demangle::demangle;
            *name_to_size.entry(demangle(&symbol.name).to_string())
                .or_insert(0) += symbol.size as isize;
        }
    }

    SizeData { name_to_size, data_size, rodata_size }
}

fn main() {
    let cmdline_matches = clap::App::new("size_diff")
        .arg(clap::Arg::with_name("before")
            .help("Base ELF file to diff")
            .required(true))
        .arg(clap::Arg::with_name("after")
            .help("ELF file to diff relative to `before`")
            .required(true))
        .get_matches();

    let before = read_elf(cmdline_matches.value_of("before")
        .expect("`before` binary not specified"));
    let after = read_elf(cmdline_matches.value_of("after")
        .expect("`after` binary not specified"));

    // Vector of symbols that were added in `after` (i.e. present in `after` but
    // not `before`). These are stored as a (size, name) tuple, so that sorting
    // the vector sorts first by size and secondly by name.
    let mut added_syms = Vec::new();
    for (name, &size) in &after.name_to_size {
        if before.name_to_size.contains_key(name) { continue; }
        added_syms.push((size, name));
    }

    // Collect symbols whose size changed as well as symbols that were removed.
    // The size values are a delta, so the removed symbols have a negative
    // "size".
    let mut changed_syms = Vec::new();
    let mut removed_syms = Vec::new();
    for (name, &size) in &before.name_to_size {
        if let Some(&after_size) = after.name_to_size.get(name) {
            if size == after_size { continue; }
            changed_syms.push((after_size - size, name)); 
        } else {
            removed_syms.push((-size, name));
        }
    }

    // Sort the three diff groups.
    added_syms.sort_unstable();
    changed_syms.sort_unstable();
    removed_syms.sort_unstable();

    // Display the symbol deltas, accumulating the total difference as we go.
    let mut total_delta = 0;
    for (delta, sym) in &added_syms {
        println!("Added {}, {:+}", sym, delta);
        total_delta += delta;
    }
    for (delta, sym) in &changed_syms {
        println!("Changed {}, {:+}", sym, delta);
        total_delta += delta;
    }
    for (delta, sym) in &removed_syms {
        println!("Removed {}, {:+}", sym, delta);
        total_delta += delta;
    }

    // Also give the .data and .rodata deltas, if they're present.
    if before.data_size != 0 || after.data_size != 0 {
        let delta = after.data_size - before.data_size;
        total_delta += delta;
        println!(".data delta: {:?}", delta);
    }
    if before.rodata_size != 0 || after.rodata_size != 0 {
        let delta = after.rodata_size - before.rodata_size;
        total_delta += delta;
        println!(".rodata delta: {:?}", delta);
    }

    // Last, display the total.
    println!("Total delta: {:?}", total_delta);
}
