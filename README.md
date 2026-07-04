# The SC2 replay format

The StarCraft 2 replay format is a modified MoPaQ (MPQ) archive containing few files. The replay format add a custom header on top of the standard MPQ format.

Disclaimer: this document isn't a complete documentation of the MoPaQ format and may not even be exhaustive for the SC2 repay format. It's a summary of the information I've used to write my parser so it only contains information about the MoPaQ format related to SC2's replay file. On top of that, I might have skipped some information about the SC2 replay format that I haven't found useful for either parsing the or exploiting the data in it.

The layout of a replay can be schematized this way:

```
+------------+------------+--------------------------------------------+
|            |            |                                            |
| SC2 Header | MPQ Header | content                                    |
|            |            |                                            |
+------------+------------+--------------------------------------------+
```

The SC2 archive is encoded using little endianness.

## SC2 header

The layout of the SC2 header is the following :
<table style=" border: 1px solid black;">
<tr><td style="border: 1px solid black;">Bytes</td><td style="text-align: center;">Description</td></tr>

<tr><td style="border: 1px solid black;">0x4d</td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;"> Magic Number</td></tr>
<tr> <td style="border: 1px solid black;">0x50</td>  </tr>
<tr> <td style="border: 1px solid black;">0x51</td>  </tr>
<tr> <td style="border: 1px solid black;">0x1b</td>  </tr>

<tr><td style="border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;"> Offset (4 bytes)</td></tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>

<tr><td style="border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;"> Size (4 bytes)</td></tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>

<tr><td style="border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;"> data (variable length)</td></tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;">........</td>  </tr>

</table>
with:

- `magic number`: `0x4d 0x50 0x51 0x1b`
- `max size`: the max size of the SC2 header (FIXME recheck)
- `offset`: the offset between the SC2 header and the MPQ header
- `size`: the size of the SC2 header
- `data`: the content of the header stored in a format described below

### Header data format

The header data is using a simple format where the data type is encoded in a byte at the beginning of the data.

The types of data encoded in this format are the following:

| Data type              | Description                        | format                                               | variables                                                                                         |
|------------------------|------------------------------------|------------------------------------------------------|---------------------------------------------------------------------------------------------------|
| bytes string           | A windows 1252 encoded string      | `[0x02 | size (1 byte) | data... ]`                  | - `size` the size in bytes of the string <br>- `data` the string encoded in windows 1252 format   |
| array                  | an array of values                 | `[0x04 | 0x10 | 0x00 | size (1 bytes) | [data]... ]` | - `size` the size in bytes of the string <br>- `data` values encoded in the format described here |
| key/value table | an association table using 8 bits integer as keys |`[0x5 | size (1 byte) | [key: 1 byte | value]... ]`| - `size` the number of elements in the table<br>- `key` the 8 bit integer key<br>- `value` the value encoded using the format described here |
| single byte integer    | a 8 bits unsigned integer          | `[0x06 | number (1 byte) ]`                          |                                                                                                   |
| four bytes integer     | a 32 bits unsigned integer         | `[0x07 | number (4 bytes) ]`                         |                                                                                                   |
| Variable Length Format | a variable length unsigned integer | `[0x09 | numbers...]`                                | the number is stored in a sequence of 7 bits integer with the 8th bit being a flag that indicate whether the current integer is the last one|

#### Examples

##### Single byte integer:
```
0x06 2C
```


##### Four bytes integer:
```
0x07 2C 45 3A 41
```


##### Bytes string:
```
0x02 0x16 0x53 0x74 0x61 0x72 0x43 0x72 0x61 0x66 0x74 0x20 0x49 0x49 0x20 0x72 0x65 0x70 "6c", 0x61 0x79 "1b", 0x31 0x31
```
which is decoded to:
```
StarCraft II replay\u{1b}11
```

##### Array:
```
0x04 0x03 0x04 2C 0x05 2C 45 3A 41 0x02 0x16 0x53 0x74 0x61 0x72 0x43 0x72 0x61 0x66 0x74 0x20 0x49 0x49 0x20 0x72 0x65 0x70 "6c", 0x61 0x79 "1b", 0x31 0x31
```
which is an array containing the 3 examples above


##### Key/value table:
```
0x04 0x03 0x01 0x04 2C 0x02 0x05 2C 45 3A 41 0x03 0x02 0x16 0x53 0x74 0x61 0x72 0x43 0x72 0x61 0x66 0x74 0x20 0x49 0x49 0x20 0x72 0x65 0x70 "6c", 0x61 0x79 "1b", 0x31 0x31
```
which is the same as the array above but with the indexes used an key to the key/value table


##### Variable length integer:
```
0x09 0b10001100 0b10000011 0b10000000 0b00000001
```

#### Content of the header's data

I haven't found documentation on the meaning of the structured data in the SC2 header. As far as I know, the only relevant content for parsing is the size and the offset.

## MPQ archive

The MPQ archive is composed of four different parts:
- The MPQ header
- The hash table
- The block table
- The content of the archive

### MPQ Headder

The MPQ header starts at the end of the SC2 Header plus the offset defined in the SC2 Header.
All the offset in the rest of the file are defined from the start of the MPQ header.

The layout of the MPQ header is the following
<table style=" border: 1px solid black;">
<tr><td style=" border: 1px solid black;">Bytes</td><td style="text-align: center;">Description</td></tr>

<tr><td style=" border: 1px solid black;">0x4d</td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;"> Magic Number</td></tr>
<tr> <td style=" border: 1px solid black;">0x50</td>  </tr>
<tr> <td style=" border: 1px solid black;">0x51</td>  </tr>
<tr> <td style=" border: 1px solid black;">0x1a</td>  </tr>

<tr><td style="border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;"> Header size (4 bytes)</td></tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>

<tr><td style="border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;"> Archive size (4 bytes)</td></tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>

<tr><td style="border: 1px solid black;"></td> <td rowspan="2" style="text-align: center; vertical-align: middle; padding-right:10px;"> format version (2 bytes)</td></tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>

<tr><td style="border: 1px solid black;"></td> <td rowspan="1" style="text-align: center; vertical-align: middle; padding-right:10px;"> sector size shift (1 byte)</td></tr>

<tr><td style="border: 1px solid black;"></td> <td rowspan="1" style="text-align: center; vertical-align: middle; padding-right:10px;"> padding byte (1 byte)</td></tr>

<tr><td style="border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;"> hashtable offset (4 bytes)</td></tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>

<tr><td style="border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;"> blocktable offset (4 bytes)</td></tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>

<tr><td style="border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;"> hashtable entries (4 bytes)</td></tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>

<tr><td style="border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;"> blocktable entries (4 bytes)</td></tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
<tr> <td style="border: 1px solid black;"></td>  </tr>
</table>

with:

- `Header size`: the size of this header.
- `Archive size`: the size of the archive without the SC2 header and without the offset between the SC2 header and the MPQ header.
- `Format version`: the version of the MPQ format, it should be always the same for all SC2 replays.
- `Sector size shift`: power of two exponent specifying the number of 512-byte disk sectors in each logical sector in the archive, the size of each logical sector in the archive is `512 * (2 ^ sector size shift)`, due to a bug in blizzard's implementation, this should always be 3.
- `Hashtable offset`/`Blocktable offset`: offset at which the blocktable/hashtable start
- `Hashtable entries`/`Blocktable entries`: the number of entries in the blocktable/hashtable

## Blocktable

The bloctable is a table containing information about each region of the archive (typically files but also empty region of the archive).

The blocktable is encrypted using the string `(block table)` hashed using the `hash file key` method.

The layout of an entry of the blocktable is the following:
<table style=" border: 1px solid black;">
<tr><td style=" border: 1px solid black;">Bytes</td><td style="text-align: center;">Description</td></tr>

<tr><td style=" border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;">Offset (4 bytes)</td></tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>

<tr><td style=" border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;">Block size (4 bytes)</td></tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>

<tr><td style=" border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;">File size (4 bytes)</td></tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>

<tr><td style=" border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;">Flags (4 bytes)</td></tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>

</table>

with:

- `Offset` the offset at which the block begin, relative to the beginning of the archive (not counting the SC2 header).
- `Block size` the size of the block in the archive.
- `File size` the size of the file which can be the same as `Block size` in case of an uncompressed file. If the block isn't a file this value is meaningless and should be 0.
- `Flags` flags for the block:
  - `0x80000000`: the block is a file and follow the file data format, otherwise the block is free or unused and `File size` should be 0.
  - `0x01000000`: the file is stored as a single unit, otherwise the file is split into multiple blocks.
  - `0x00020000`: the file's encryption key is adjusted by the block offset and file size (explained in detail in the File Data section). File must be encrypted.
  - `0x00010000`: the file is encrypted.
  - `0x00000200`: the file is compressed, file cannot be imploded.
  - `0x00000100`: the file is imploded, file cannot be compressed.

The blocktable can be extended to support files larger than 4 GB but this doesn't concern SC2 replays.


## Hashtable

The hashtable stores the index of the blocktable entry corresponding to the file. Instead of using file names as keys of the table, it use two different hash of the file name. On top of the blocktable entry index, a hashtable entry also contains the language of the file, using windows `LANGID` data type, and a seamingly unused entry for the platform of the file.

The hashtable is encrypted using the string `(hash table)` hashed using the `hash file key` method.

The layout of an entry of the hashtable is the following:

<table style=" border: 1px solid black;">
<tr><td style=" border: 1px solid black;">Bytes</td><td style="text-align: center;">Description</td></tr>

<tr><td style=" border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;">Filename hash A (4 bytes)</td></tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>

<tr><td style=" border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;">Filename hash B (4 bytes)</td></tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>

<tr><td style=" border: 1px solid black;"></td> <td rowspan="2" style="text-align: center; vertical-align: middle; padding-right:10px;">Language (2 bytes)</td></tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>

<tr><td style=" border: 1px solid black;"></td> <td rowspan="1" style="text-align: center; vertical-align: middle; padding-right:10px;">Platform (1 byte)</td></tr>

<tr><td style=" border: 1px solid black;"></td> <td rowspan="1" style="text-align: center; vertical-align: middle; padding-right:10px;">Padding byte (1 byte)</td></tr>

<tr><td style=" border: 1px solid black;"></td> <td rowspan="4" style="text-align: center; vertical-align: middle; padding-right:10px;">Blocktable index (4 bytes)</td></tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>
<tr> <td style=" border: 1px solid black;"></td>  </tr>

</table>


with:

- `Filename hash A` the hashed value of the filename using method A
- `Filename hash B` the hashed value of the filename using method B
- `Language` the language of the file, using windows' `langid` format
- `Platform` the platform value (unused)
- `Blocktable index` the index of the entry corresponding to this file in the blocktable

## Hashing and encryption algorithm

### Crypt table

The MoPaQ archive use custom algorithms for hashing and encryption/decryption. Those algorithms use a crypt table.
This crypt table is an array of size `0x500` and initalized this way:

```rust
let mut table =  [0u64; 0x500];

let mut seed = 0x00100001;

for index1 in 0..0x100 {
    let mut index2 = index1;
    for _ in 0..5 {
        seed  = (seed * 125 + 3) % 0x2AAAAB;
        let temp1 = (seed & 0xFFFF) << 0x10;

        seed  = (seed * 125 + 3) % 0x2AAAAB;
        let temp2 = seed & 0xFFFF;

        table[index2] = temp1 | temp2;

        index2 += 0x100
    }
}
```

### Hashing

The hashing algorithm has different "methods". Those methods are an integer used in the hashing algorithm.

```rust
pub enum HashType {
    MPQHashtableOffset = 0,
    MPQHashNameA       = 1, // Hash method A
    MPQHashNameB       = 2, // Hash method B
    MPQHashFileKey     = 3, // Hash file key method
}
```

The hashing algorithm is the following:

```rust
pub fn hash(&self, string: String, hashtype: HashType) -> u64 {
    let mut seed1: u64 = 0x7FED7FED;
    let mut seed2: u64 = 0xEEEEEEEE;
    let hashtype = hashtype as u64;

    for c in string.to_uppercase().chars() {
        let c: u64 = c.into();
        let value = self.dw_crypt_table[((hashtype << 8) + c) as usize];
        seed1 = (value ^ seed1.wrapping_add(seed2)) & 0xFFFFFFFF ;
        seed2 = c.wrapping_add(seed1).wrapping_add(seed2).wrapping_add(seed2 << 5).wrapping_add(3) & 0xFFFFFFFF;
    }
    seed1
}
```
Here the hash function is implemented as a member of the `CryptTable` class and the crypt table is the `dw_crypt_table` member.

### Decryption

Similarly to the hash function, the decryption function is implemented as a member of the `CryptTable`.

```Rust
pub fn decrypt(&self, buffer: &[u64], key: u64) -> Vec<u64> {

    let mut res = vec![0; buffer.len()];

    let mut seed1: u128 = key as u128;
    let mut seed2: u128 = 0xEEEEEEEE;
    let mask: u128 = 0xFFFFFFFF; // (1 << 64) - 1;

    for (i, d) in buffer.iter().enumerate() {
        seed2 += self.dw_crypt_table[0x400usize + ((seed1 as usize) & 0xFFusize)] as u128;
        seed2 &= mask;

        let mut val = *d as u128;
        val ^= seed1 + seed2;
        val &= mask;

        seed1 = ((!seed1 << 0x15) + 0x11111111) | (seed1 >> 0x0B);
        seed1 &= mask;

        seed2 = val + seed2 + (seed2 << 5) + 3;
        seed2 &= mask;

        res[i] = val as u64
    }
    res
}
```

The decryption key is often a hash of a hardcoded string (cf blocktable and hashtable).
