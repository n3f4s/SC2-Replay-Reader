extern crate encoding;
extern crate num_bigint;

mod mpq;
mod sc2mpq;
mod utils;
mod sc2;

use std::{ env, fs, collections::HashMap };

use mpq::{ parser::*, structs::*, utils::* };
use utils::*;

use num_bigint::{ BigInt, BigUint, ToBigInt };

macro_rules! typed_get {
    ($hash:ident, $idx:literal, $rtype:path, $expr:expr) => {
        match &**$hash.get(&$idx.to_bigint().unwrap()).unwrap() {
            $rtype(value) => $expr(value),
            v => panic!("Expected {}, got {:?} for {} at index {}",
                        stringify!($rtype), v, stringify!($hash), $idx),
        }
    };
    ($hash:ident, $idx:literal, $rtype:path) => {
        match &**$hash.get(&$idx.to_bigint().unwrap()).unwrap() {
            $rtype(value) => value,
            v => panic!("Expected {}, got {:?} for {} at index {}",
                        stringify!($rtype), v, stringify!($hash), $idx),
        }
    };
    ($hash:ident, $idx:expr, $rtype:path) => {
        match &**$hash.get(&$idx).unwrap() {
            $rtype(value) => value,
            v => panic!("Expected {}, got {:?} for {} at index {}",
                        stringify!($rtype), v, stringify!($hash), $idx),
        }
    };
    ($hash:ident, $idx:expr, $rtype:path, $expr:expr) => {
        match &**$hash.get(&$idx).unwrap() {
            $rtype(value) => $expr(value),
            v => panic!("Expected {}, got {:?} for {} at index {}",
                        stringify!($rtype), v, stringify!($hash), $idx),
        }
    };
}

macro_rules! typed_expect {
    ($val:expr, $rtype:path, $expr:expr) => {
        match $val {
            $rtype(value) => $expr(value),
            v => panic!("Expected {} : {:?}", stringify!($rytpe), v),
        }
    };
    ($val:expr, $rtype:path) => {
        match $val {
            $rtype(value) => value,
            v => panic!("Expected {} : {:?}", stringify!($rytpe), v),
        }
    };
}

fn decode_blob(v: &[u8]) -> String { decode_utf8(v).unwrap() }

fn parse_color_hashmap(a: &HashMap<BigInt, Box<sc2::structs::DataType>>) -> HashMap<u64, BigInt> {
    fn bigint_to_u64(v: &BigInt) -> u64 {
        if v > &BigInt::ZERO {
            v.to_u64_digits().1[0]
        } else {
            0
        }
    }
    fn map((k, v): (&BigInt, &Box<sc2::structs::DataType>)) -> (u64, BigInt) {
        typed_expect!(
            &**v,
            sc2::structs::DataType::VInt,
            |i: &BigInt| ( bigint_to_u64(&k), i.clone())
        )
    }
    a.iter().map(map).collect()


}

fn read_sc2mpq_header(data: &[u8]) -> (&[u8], SC2MPQHeader, usize) {
    let total_size = data.len();
    let (data, sc2header) = parse_sc2mpq_header(&data).unwrap();
    let header_size = total_size - data.len();
    let data = jump_to_offset(sc2header.header_offset as usize, total_size, data.len(), data);
    (data, sc2header, header_size)
}

fn read_mpq_header(data: &[u8]) -> (&[u8], MPQHeader, usize) {
    let total_size = data.len();
    let (data, mpqheader) = parse_mpq_header(&data).unwrap();
    let header_size = total_size - data.len();
    (data, mpqheader, header_size)
}

fn read_blocktable(data: &[u8], crypttable: &CryptTable, mpqheader: &MPQHeader) -> Vec<BlockTableEntry> {
    let blocktable_offset = mpqheader.blocktable_offset as usize;
    let blocktable_end = blocktable_offset + mpqheader.blocktable_byte_size() as usize;

    let encrypted_blocktable = &data[blocktable_offset..blocktable_end];
    let blocktable_key = crypttable.hash("(block table)".to_string(), HashType::MPQHashFileKey);
    let blocktable = crypttable.decrypt(&pack_u8_vec(encrypted_blocktable), blocktable_key);

    let unpacked_blocktable = unpack_u64_vec(&blocktable);
    let ( _, parsed_blocktable ) = parse_blocktable(&unpacked_blocktable, mpqheader.blocktable_entries as usize).unwrap();

    parsed_blocktable
}

fn read_hashtable(data: &[u8], crypttable: &CryptTable, mpqheader: &MPQHeader) -> Vec<HashTableEntry> {
    let hashtable_offset = mpqheader.hashtable_offset as usize;
    let hashtable_end = hashtable_offset + mpqheader.hashtable_byte_size() as usize;

    let encrypted_hashtable = &data[hashtable_offset..hashtable_end];
    let hashtable_key = crypttable.hash("(hash table)".to_string(), HashType::MPQHashFileKey);
    let hashtable = crypttable.decrypt(&pack_u8_vec(encrypted_hashtable), hashtable_key);

    let unpacked_hashtable = unpack_u64_vec(&hashtable);
    let ( _, parsed_hashtable ) = parse_hashtable(&unpacked_hashtable, mpqheader.hashtable_entries as usize).unwrap();

    parsed_hashtable
}

fn find_and_read_file(data: &[u8], file: String, hashtable: &Vec<HashTableEntry>,
                      crypttable: &CryptTable,
                      blocktable: &Vec<BlockTableEntry>,
                      mpqheader: &MPQHeader) -> Result<Vec<u8>, String> {
    let file_ht = find_hash_entry_by_name(file, &hashtable, &crypttable).unwrap();
    match file_ht.file_block_index {
        FileBlockIndex::FileMissing(FileMissingFlag::Empty) => Err(String::from("File Missing")),
        FileBlockIndex::FileMissing(FileMissingFlag::Deleted) => Err(String::from("File Deleted")),
        FileBlockIndex::FilePresent(i) => {
            let bt_entry = &blocktable[i as usize];
            println!("offset : {:#x}", bt_entry.block_offset);
            match read_file(data, &mpqheader, &bt_entry, &file_ht, false) {
                Err(FileReadError::EncryptionNotImplemented) => Err(String::from("Encrypted file")),
                Err(FileReadError::NotAFile) => Err(String::from("Not a file")),
                Err(FileReadError::ZeroSizedFile) => Err(String::from("File size is null")),
                Err(FileReadError::UnknownCompression(i)) => Err(format!("Unknown compression format {}", i)),
                Err(FileReadError::FailedDecompression) => Err(String::from("Failed to decompress the file")),
                Ok(file) => Ok(file)
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let args: Vec<String> = env::args().collect();
    let data: Vec<u8> = fs::read(args[1].clone())?;

    let (data, sc2header, _sc2_header_size) = read_sc2mpq_header(&data);
    println!("{}", sc2header);

    let (_, mpqheader, _mpq_header_size) = read_mpq_header(data);
    println!("{}", mpqheader);

    let crypttable = CryptTable::new();

    let blocktable = read_blocktable(data, &crypttable, &mpqheader);
    println!("Blocktable:");
    println!("offset (dec)\tblock size\tfile size\tflags");
    for block in &blocktable {
        println!("{:#x} ({})\t{}\t{}\t{:x}", block.block_offset, block.block_offset, block.block_size, block.file_size, block.flags);
    }

    let hashtable = read_hashtable(data, &crypttable, &mpqheader);
    println!("Hashtable:");
    println!("Hash A\tHash B\tLocl\tPlat\tBlockIdx");
    for entry in &hashtable {
        println!("{:#x}\t{:#x}\t{:#x}\t{:#x}\t{:#x}\t",
                 entry.filepath_hash_a,
                 entry.filepath_hash_b,
                 entry.language,
                 entry.platform_id,
                 match entry.file_block_index {
                     FileBlockIndex::FilePresent(i) => i,
                     FileBlockIndex::FileMissing(i) => i.into(),
                 }
        );
    }

    let mut file_list = Vec::<String>::new();
    let file_list_ht = find_hash_entry_by_name("(listfile)".to_string(), &hashtable, &crypttable).unwrap();
    match file_list_ht.file_block_index {
        FileBlockIndex::FileMissing(FileMissingFlag::Empty) => println!("File Missing"),
        FileBlockIndex::FileMissing(FileMissingFlag::Deleted) => println!("File Deleted"),
        FileBlockIndex::FilePresent(i) => {
            let bt_entry = &blocktable[i as usize];
            println!("offset : {:#x}", bt_entry.block_offset);
            match read_file(data, &mpqheader, &bt_entry, &file_list_ht, false) {
                Err(FileReadError::EncryptionNotImplemented) => println!("Encrypted file"),
                Err(FileReadError::NotAFile) => println!("Not a file"),
                Err(FileReadError::ZeroSizedFile) => println!("File size is null"),
                Err(FileReadError::UnknownCompression(i)) => println!("Unknown compression format {}", i),
                Err(FileReadError::FailedDecompression) => println!("Failed to decompress the file"),
                Ok(file) => {
                    match decode(&file) {
                        Ok(f) => {
                            file_list = f.split("\r\n").map(String::from).collect();
                            println!("\n{:?}\n", file_list);
                        },
                        Err(_) => println!("Failed to decode file"),
                    }
                },
            }
        }
    }
    println!("File list : {:?}", file_list);
    let file_data = find_and_read_file(data, String::from("replay.details"), &hashtable,
                                       &crypttable, &blocktable, &mpqheader).unwrap();
    let (_left_over, (parsed, _)) = sc2::parser::parse_struct(&file_data, 0).unwrap();
    println!("{}", parsed);
    // FIXME : add version dependant things
    let details =  typed_expect!(parsed, sc2::structs::DataType::HashMap);
    println!("{{");

    let map_name = typed_get!(details, 1, sc2::structs::DataType::Blob, decode_blob);
    println!("    map name : {}", map_name);

    let difficulty = typed_get!(details, 2, sc2::structs::DataType::Blob, decode_blob);
    println!("    difficulty : \"{}\"", difficulty);

    let thumbnail = typed_get!(details, 3, sc2::structs::DataType::HashMap);
    let thumbnail = typed_get!(thumbnail, 0, sc2::structs::DataType::Blob, decode_blob);
    println!("    thumbnail : \"{}\"", thumbnail);

    let blizz_map = typed_get!(details, 4, sc2::structs::DataType::UInt8);
    println!("    blizzard map : {}", blizz_map);

    let file_time = typed_get!(details, 5, sc2::structs::DataType::VInt);
    println!("    file time : {}", file_time);

    let utc_adjustment = typed_get!(details, 6, sc2::structs::DataType::VInt);
    println!("    UTC adjustment : {}", utc_adjustment);

    let description = typed_get!(details, 7, sc2::structs::DataType::Blob, decode_blob);
    println!("    description : {}", description);

    let image_file_path = typed_get!(details, 8, sc2::structs::DataType::Blob, decode_blob);
    println!("    image file path : {}", image_file_path);

    let map_file_name = typed_get!(details, 9, sc2::structs::DataType::Blob, decode_blob);
    println!("    map file name : {}", map_file_name);

    let cache_handle = typed_get!(details, 10, sc2::structs::DataType::Array);
    let cache_handle = cache_handle.iter().map(
        |v| {
            let bytes = typed_expect!(&**v, sc2::structs::DataType::Blob);
            let mut server = decode_utf8(&bytes[4..8]).unwrap().trim_matches(|c| c == '\x00' || c == ' ').to_lowercase();
            let mut res: Vec<u8> = Vec::new();
            for e in &bytes[8..] {
                let s = format!("{:02x}", e);
                res.push(s.bytes().nth(0).unwrap());
                res.push(s.bytes().nth(1).or(Some(0)).unwrap());
            }
            let hash = decode_utf8(&res).unwrap();
            let type_ = decode_utf8(&bytes[0..4]).unwrap();
            let mut scheme = "https";
            let mut domain = "classic.blizzard.com";

            if server == "sea" {
                server = "us".to_string();
            } else if server == "cn" {
                scheme = "http";
                domain = "battlenet.com.cn";
            };

            format!("{}://{}-s2-depot.{}/{}.{}", scheme, server, domain, hash, type_)

        }
    ).collect::<Vec<String>>();
    println!("    cache handle : {:?}", cache_handle);

    let mini_save = typed_get!(details, 11, sc2::structs::DataType::UInt8);
    println!("    mini save : {}", mini_save);

    let game_speed = typed_get!(details, 12, sc2::structs::DataType::VInt);
    println!("    game speed : {}", game_speed);

    let default_difficulty = typed_get!(details, 13, sc2::structs::DataType::VInt);
    println!("    default difficulty : {}", default_difficulty);

    let players =  typed_get!(details, BigInt::ZERO, sc2::structs::DataType::Array);
    for player in players {
        let player =   typed_expect!(&**player, sc2::structs::DataType::HashMap);
        let name =     typed_get!(player, BigInt::ZERO, sc2::structs::DataType::Blob,    decode_blob);
        let bnet =     typed_get!(player, 1,            sc2::structs::DataType::HashMap,
                                  |hm: &HashMap<BigInt, Box<sc2::structs::DataType>>| (
                                      typed_get!(hm, 0, sc2::structs::DataType::VInt).clone(),
                                      typed_get!(hm, 1, sc2::structs::DataType::UInt32).clone(),
                                      typed_get!(hm, 2, sc2::structs::DataType::VInt).clone(),
                                      // Documented but never available
                                      // typed_get!(hm, 3, sc2::structs::DataType::VInt).clone(),
                                      typed_get!(hm, 4, sc2::structs::DataType::VInt).clone(),
                                  )
        );
        let race =     typed_get!(player, 2,            sc2::structs::DataType::Blob,    decode_blob);
        let color =    typed_get!(player, 3,            sc2::structs::DataType::HashMap, parse_color_hashmap);
        let control =  typed_get!(player, 4,            sc2::structs::DataType::VInt);
        let team =     typed_get!(player, 5,            sc2::structs::DataType::VInt);
        let handicap = typed_get!(player, 6,            sc2::structs::DataType::VInt);
        let observe =  typed_get!(player, 7,            sc2::structs::DataType::VInt);
        let result =   typed_get!(player, 8,            sc2::structs::DataType::VInt);

        println!("    {{
        name: {},
        race: {},
        colour: {{ a: {}, r: {}, g: {}, b: {} }},
        control: {},
        handicap: {},
        observe: {},
        result: {};
        bnet: {{ region : {}, program_id : {}, subregion : {}, uid : {} }};
        team: {};
    }}",
                 name,
                 race,
                 color[&0], color[&1], color[&2], color[&3],
                 control,
                 handicap,
                 observe,
                 result,
                 bnet.0, bnet.1, bnet.2, bnet.3,
                 team,
        )
    }
    println!("{{");
    Ok(())
}
