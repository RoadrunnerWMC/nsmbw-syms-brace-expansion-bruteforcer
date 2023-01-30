use std::collections::HashMap;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

use brace_expand_2::brace_expand_iter;
use djb2_utils::{hash_djb2, DJB2_HASH_SEED};
use lazy_static::lazy_static;
use nvidia_demangle::demangle;
use regex::Regex;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use symbol_map_formats::{BasicSymbolMap, load_symbol_map_from_path};


const ONLY_ECHO_FIRST: usize = 50;
const ALSO_ECHO_EVERY: usize = 2_000_000;


fn make_pattern_shorthands() -> HashMap<String, String> {
    let primitives = "{,P,R,C}{{,U,S}{c,s,i,l,f},b}";
    HashMap::from([
        ("END1".to_owned(), format!("{{C,}}F{{v,{primitives}}}", )),
        ("END2".to_owned(), format!("{{C,}}F{{v,{primitives},{primitives}{primitives}}}")),
        ("END3".to_owned(), format!("{{C,}}F{{v,{primitives},{primitives}{primitives},{primitives}{primitives}{primitives}}}")),
    ])
}


#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct SymbolDatabaseEntry {
    pub address: u32,
    pub mangled_hash: u32,
    pub demangled_hash: u32,
    pub mangled_name: Option<String>,
}


#[derive(Clone, Eq, PartialEq, Debug)]
struct SymbolDatabase {
    /// mangled hash -> demangled hash -> symbols with those hashes.
    /// Organized this way so we can efficiently check if there are any
    /// symbols matching a mangled hash, so we can skip demangling
    /// altogether if not.
    pub contents: HashMap<u32, HashMap<u32, Vec<SymbolDatabaseEntry>>>,
}


impl SymbolDatabase {
    fn new(basic_map: &BasicSymbolMap) -> Self {
        lazy_static! {
            static ref HASHNAME_REGEX: Regex = Regex::new(concat!(
                r"^",                 // (start of string)
                r"hashname_",         // "hashname_"
                r"([a-fA-F0-9]{8})",  // hex number
                r"_",                 // underscore
                r"([a-fA-F0-9]{8})",  // hex number
            )).unwrap();
        }

        let mut map: HashMap<u32, HashMap<u32, Vec<SymbolDatabaseEntry>>> = HashMap::new();
        for (address, name) in basic_map.iter() {
            let (mangled_hash, demangled_hash, mangled_name) = if let Some(caps) = HASHNAME_REGEX.captures(name) {
                let mangled_hash = caps.get(1).unwrap().as_str();
                let demangled_hash = caps.get(2).unwrap().as_str();

                // These are guaranteed to succeed because the regex
                // only allows hex digits for them
                (u32::from_str_radix(mangled_hash, 16).unwrap(),
                 u32::from_str_radix(demangled_hash, 16).unwrap(),
                 None)
            } else {
                (hash_djb2(name.as_bytes(), DJB2_HASH_SEED),
                 hash_djb2(demangle(name).unwrap_or_else(|_| "ERROR".to_owned()).as_bytes(), DJB2_HASH_SEED),
                 Some(name.clone()))
            };

            map.entry(mangled_hash).or_default().entry(demangled_hash).or_default().push(SymbolDatabaseEntry{
                address: *address,
                mangled_hash,
                demangled_hash,
                mangled_name,
            });
        }
        Self{contents: map}
    }
}


/// Replaces any "[" "]" pairs with length prefixes, in-place.
fn apply_square_bracket_length_prefix_substitution(s: &mut String) {
    // We search for "[" in reverse and "]" forward, instead
    // of the other way around, because we have to process
    // these from innermost to outermost if they're nested
    // (or else we'll insert incorrect length values)

    // TODO: it should be possible to optimize this further (go over
    // the string in one pass instead of multiple)

    while let Some(open_bracket_byte_idx) = s.find('[') {
        if let Some(close_bracket_byte_idx) = s.rfind(']') {
            let substring_length = close_bracket_byte_idx - open_bracket_byte_idx - 1;
            s.remove(close_bracket_byte_idx);
            s.replace_range(
                open_bracket_byte_idx..open_bracket_byte_idx+1,
                &substring_length.to_string());
        } else {
            // TODO: um...?
            break;
        }
    }
}


/// Applies the global pattern-shorthand replacements and returns a new
/// String.
fn apply_pattern_shorthands(s: &str) -> String {
    let s = s.to_owned();
    make_pattern_shorthands().iter().fold(s, |acc, kv| acc.replace(kv.0, kv.1))
    // let s2 = None;
    // for name, replacement in make_pattern_shorthands() {
    //     if let Some(s2_inner) = s2 {
    //         s2 = Some(Some(s.replace(name, replacement)))
    //     }
    //     s2 = ;
    // }
    // s.clear();
    // s.push_str(s_new);
}


fn process_line_as_pattern(line: &str, db: &mut SymbolDatabase) {
    let line = apply_pattern_shorthands(line);

    let mut iter = brace_expand_iter(&line, false);
    let num_expansions = iter.num_expansions();

    if num_expansions > ONLY_ECHO_FIRST {
        println!("Checking {num_expansions} symbols...");
    }

    let query_start_time = Instant::now();

    let mut sym_mangled = String::with_capacity(iter.max_expansion_length());
    let mut next_i = 0;
    let mut echo_interval_jitter = 0;
    let mut newly_found_syms = Vec::new();
    while iter.next_into(&mut sym_mangled) {
        // (doing it this way so we can safely `continue` in the middle
        // of this loop if we want to)
        let i = next_i;
        next_i += 1;

        let mut force_echo = i < ONLY_ECHO_FIRST || (i + echo_interval_jitter) % ALSO_ECHO_EVERY == 0;

        apply_square_bracket_length_prefix_substitution(&mut sym_mangled);

        let hash_mangled = hash_djb2(sym_mangled.as_bytes(), DJB2_HASH_SEED);

        let matching_mangled_db = db.contents.get(&hash_mangled);

        // Important optimization
        if matching_mangled_db.is_none() && !force_echo {
            continue;
        }

        let sym_demangled = demangle(&sym_mangled).unwrap_or_else(|_| "ERROR".to_string());
        let hash_demangled = hash_djb2(sym_demangled.as_bytes(), DJB2_HASH_SEED);

        let mut status = "";
        let mut new_unknown_syms = None;
        if let Some(matching_mangled_db) = matching_mangled_db {
            if let Some(matching_both_db) = matching_mangled_db.get(&hash_demangled) {
                // There are symbols matching both of the hashes. Pick out the ones with unknown names
                let thing: Vec<&SymbolDatabaseEntry> = matching_both_db.iter().filter(|sym| sym.mangled_name.is_none()).collect();
                if thing.is_empty() {
                    status = "(known)";
                } else {
                    new_unknown_syms = Some(thing);
                    force_echo = true;
                    status = "!!!!!!!!!!!!!!!!! ";  // (18 "!" + 1 " ")
                }
            }
        }

        if new_unknown_syms.is_none() && !force_echo {
            continue;
        }

        println!("{hash_mangled:08x}_{hash_demangled:08x} | {sym_mangled}");
        println!("{status:^18}| {sym_demangled}");

        if i == ONLY_ECHO_FIRST - 1 {
            println!("For performance, only the first {ONLY_ECHO_FIRST} symbols are displayed (above), plus a small sample of the rest (below):");
        }

        if force_echo {
            // We add a bit of jitter to the echo interval because
            // otherwise it can end up being a multiple of some
            // sub-pattern in the brace-expansion output sequence, which
            // causes us to only show some types of outputs and not a
            // more representative sample.
            echo_interval_jitter += 1;
        }

        if let Some(new_unknown_syms) = new_unknown_syms {
            println!("{empty:^>width$}", empty = "", width = 70);  // ("^" * 70)

            for matching_sym in new_unknown_syms {
                newly_found_syms.push((matching_sym.clone(), sym_mangled.clone(), sym_demangled.clone()));

                if let Ok(mut file) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("positive_symbol_log.txt") {
                    writeln!(file, "{sym_mangled}").ok();
                }
            }
        }
    }

    let symbols_checked_str = format!("({} symbol{} checked)",
        num_expansions,
        if num_expansions == 1 {""} else {"s"});

    if newly_found_syms.is_empty() {
        println!("No new matches {symbols_checked_str}.");
    } else {
        println!();
        println!("{empty:!>width$}", empty = "", width = 70);  // ("!" * 70)

        println!("Found {} new match{} {}!",
            newly_found_syms.len(),
            if newly_found_syms.len() == 1 {""} else {"es"},
            symbols_checked_str);

        for (sym, name_mangled, name_demangled) in &newly_found_syms {
            println!("{:08x} | {:<40} | {}", sym.address, name_mangled, name_demangled);
        }

        println!("{empty:!>width$}", empty = "", width = 70);  // ("!" * 70)
        println!();
    }

    let query_time = (Instant::now() - query_start_time).as_secs_f64();
    if query_time > 10.0 {
        println!("(Query executed in {query_time:0.3} seconds)");
    }
}


fn load_symbol_database_from_path(path: &Path, verbose: bool) -> Result<SymbolDatabase, Box<dyn Error>> {
    let symbol_list = load_symbol_map_from_path(path)?;

    if verbose {
        let mut file_name = "<unknown>";
        if let Some(name) = path.file_name() {
            if let Some(name) = name.to_str() {
                file_name = name;
            }
        }
        let file_name = file_name;

        let total_len = symbol_list.len();
        let unk_len = symbol_list.iter().filter(|item| item.1.starts_with("hashname_")).count();

        println!("Loaded {} symbols from {} ({} ({:0.3}%) unknown).",
            total_len, file_name, unk_len, (unk_len as f64) / (total_len as f64) * 100.0);
        println!();
    }

    Ok(SymbolDatabase::new(&symbol_list))
}


fn main() -> Result<(), Box<dyn Error>> {
    let mut rl = Editor::<()>::new()?;
    rl.load_history("history.txt").ok();

    let first_arg = std::env::args().nth(1);
    if first_arg.is_none() {
        println!("Specify the path to the symbol map as the first argument");
        return Ok(());
    }
    let first_arg = first_arg.unwrap();
    let symbol_map_path = Path::new(&first_arg);

    let mut db = load_symbol_database_from_path(symbol_map_path, true)?;

    println!("Commands:");
    println!("- Ctrl+C, Ctrl+D: exit");
    println!("- r: reload the symbol database");
    println!("- (anything else): run as a bruteforce pattern");
    println!();
    println!("Pattern format:");
    println!("- Curly braces (\"{{a,b,c}}\") expand to multiple strings (\"a\", \"b\", \"c\").");
    println!("    - Empty elements are OK: \"{{a,b,}}\" -> \"a\", \"b\", \"\".");
    println!("- Square brackets (\"[abc]\") will be replaced by a length prefix (\"3abc\").");
    println!("- You can use the following shorthand aliases to easily search for symbols with common signatures:");
    let shorthands = make_pattern_shorthands();
    let mut shorthands: Vec<(&String, &String)> = shorthands.iter().collect();
    shorthands.sort();
    for (key, value) in &shorthands {
        println!("    - \"{key}\": \"{value}\"");
    }
    println!();

    loop {
        let readline = rl.readline("sym> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                if line == "r" {
                    db = load_symbol_database_from_path(symbol_map_path, true)?;
                } else {
                    process_line_as_pattern(&line, &mut db);
                }
            },
            Err(ReadlineError::Interrupted) => {
                println!("Ctrl-C");
                break
            },
            Err(ReadlineError::Eof) => {
                println!("Ctrl-D");
                break
            },
            Err(err) => {
                println!("Error: {err:?}");
                break
            }
        }
    }

    rl.save_history("history.txt")?;
    Ok(())
}
