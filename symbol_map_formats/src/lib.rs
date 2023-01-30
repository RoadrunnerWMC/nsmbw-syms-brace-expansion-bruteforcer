pub mod dolphin;

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{Seek, Read, Write};
use std::path::Path;

use crate::dolphin::DolphinSymbolMap;


pub type BasicSymbolMap = HashMap<u32, String>;


/// Trait representing any symbol map type. Since their semantics vary a
/// lot, this just provides a common baseline of functionality common to
/// all of them. Individual implementors can generally add much richer
/// information.
pub trait SymbolMap where Self: Sized {
    /// Whether it's possible to load the map, or only save to it.
    /// If false, load() will always return Err.
    const IS_LOADABLE: bool;

    /// If None, common extensions like ".txt" and ".map" are acceptable.
    const PREFERRED_EXTENSION: Option<&'static str>;

    /// Attempts to load from a file
    fn load<SR: Seek + Read>(file: SR) -> Result<Self, Box<dyn Error>>;

    // fn from_dict_and_sections_info(map: HashMap<u32, String>, sections_info: &[HashMap<String, ???>])

    /// Converts to a HashMap {address: mangled_name}
    /// TODO: make this an Into or whatever instead?
    fn to_hashmap(&self) -> BasicSymbolMap;

    /// Writes to a file
    fn write<SW: Seek + Write>(&self, file: SW) -> Result<(), Box<dyn Error>>;

    /// Tries to detect if the file is in the symbol map format or not.
    /// Also makes a best-effort attempt to rewind it to the beginning.
    fn autodetect<SR: Seek + Read>(mut file: SR) -> bool {
        if file.rewind().is_err() {
            return false;
        }
        let res = Self::load(&mut file);
        if file.rewind().is_err() {
            return false;
        }
        res.is_ok()
    }
}


pub fn load_symbol_map_from_file<SR: Seek + Read>(mut file: SR) -> Result<BasicSymbolMap, Box<dyn Error>> {
    file.rewind()?;
    if let Ok(map) = DolphinSymbolMap::load(file) {
        return Ok(map.to_hashmap());
    }
    Err("couldn't load symbol map file".into())
}


pub fn load_symbol_map_from_path<P>(path: P) -> Result<BasicSymbolMap, Box<dyn Error>>
where P: AsRef<Path> {
    load_symbol_map_from_file(File::open(path)?)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
    }
}
