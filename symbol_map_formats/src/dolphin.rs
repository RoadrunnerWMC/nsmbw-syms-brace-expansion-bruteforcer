use std::collections::HashMap;
use std::error::Error;
use std::io::{BufReader, BufRead, Seek, Read, Write};

use lazy_static::lazy_static;
use regex::Regex;

use crate::SymbolMap;


#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct DolphinSymbolMapSymbol {
    physical_address: u32,
    size: u32,
    virtual_address: u32,
    alignment: u32,
    name: String,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct DolphinSymbolMapSection {
    name: String,
    symbols: Vec<DolphinSymbolMapSymbol>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct DolphinSymbolMap {
    sections: Vec<DolphinSymbolMapSection>,
}


impl SymbolMap for DolphinSymbolMap {
    const IS_LOADABLE: bool = true;
    const PREFERRED_EXTENSION: Option<&'static str> = None;

    fn load<SR: Seek + Read>(file: SR) -> Result<Self, Box<dyn Error>> {
        lazy_static! {
            static ref SECTION_HEADER_REGEX: Regex = Regex::new(concat!(
                r"(\S+)",            // ".text"
                r" section layout",  // " section layout"
            )).unwrap();
            static ref SYMBOL_LINE_REGEX: Regex = Regex::new(concat!(
                r"^",                       // (start of string)
                r"\s*",                     // optional leading whitespace
                r"(?P<phys>[a-fA-F0-9]+)",  // hex number
                r"\s+",                     // whitespace
                r"(?P<size>[a-fA-F0-9]+)",  // hex number
                r"\s+",                     // whitespace
                r"(?P<virt>[a-fA-F0-9]+)",  // hex number
                r"\s+",                     // whitespace
                // ---- Begin optional field ----
                r"(?:",                     // non-capturing group
                r"(?P<dol>[a-fA-F0-9]+)",   // hex number
                r"\s+",                     // whitespace
                r")??",                     // Zero or one repetitions, non-greedy
                // ---- End optional field ----
                r"(?P<align>\d+)",          // decimal number
                r"\s+",                     // whitespace
                r"(?P<name>\S+)",           // symbol name
            )).unwrap();
        }

        let mut sections = Vec::new();
        let mut current_section = None;

        for line in BufReader::new(file).lines().flatten() {
            let line = line.trim_start();
            if line.is_empty() {
                continue;
            }

            if let Some(caps) = SECTION_HEADER_REGEX.captures(line) {
                let section_name = caps.get(1).unwrap().as_str();

                if let Some(sec) = current_section {
                    sections.push(sec);
                }
                current_section = Some(DolphinSymbolMapSection{
                    name: section_name.to_owned(),
                    symbols: Vec::new(),
                });

            } else if let Some(caps) = SYMBOL_LINE_REGEX.captures(line) {
                let physical_address = caps.name("phys").unwrap().as_str();
                let size = caps.name("size").unwrap().as_str();
                let virtual_address = caps.name("virt").unwrap().as_str();
                // The regex accomodates for an optional dol_offset
                // field here, but we don't actually parse it
                let alignment = caps.name("align").unwrap().as_str();
                let name = caps.name("name").unwrap().as_str();

                // These are guaranteed to succeed because the regex
                // only allows hex digits for them
                let physical_address = u32::from_str_radix(physical_address, 16).unwrap();
                let size = u32::from_str_radix(size, 16).unwrap();
                let virtual_address = u32::from_str_radix(virtual_address, 16).unwrap();
                let alignment: u32 = alignment.parse().unwrap();  // (base 10)

                if let Some(sec) = current_section.as_mut() {
                    sec.symbols.push(DolphinSymbolMapSymbol{
                        physical_address,
                        size,
                        virtual_address,
                        alignment,
                        name: name.to_owned(),
                    });
                } else {
                    return Err(format!("{name} at {physical_address:08x} doesn't belong to any section").into());
                }
            }
        }

        if let Some(sec) = current_section {
            sections.push(sec);
        }

        Ok(DolphinSymbolMap{sections})
    }

    fn to_hashmap(&self) -> HashMap<u32, String> {
        let mut map = HashMap::new();
        for section in &self.sections {
            for symbol in &section.symbols {
                map.insert(symbol.physical_address, symbol.name.clone());
            }
        }
        map
    }

    fn write<SW: Seek + Write>(&self, _: SW) -> Result<(), Box<dyn Error>> {
        Err("not yet implemented".into())
    }
}