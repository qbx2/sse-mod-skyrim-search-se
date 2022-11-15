// Ported versionlibdb.h from https://www.nexusmods.com/skyrimspecialedition/mods/32444
use std::{fs::File, io::Read};

use byteorder::{LittleEndian, ReadBytesExt};

#[derive(thiserror::Error, Debug)]
pub enum VersionlibError {
    #[error("could not open the file: {reason}")]
    Open { reason: String },
    #[error("could not read the file: {reason}")]
    Read { reason: String },
    #[error("unexpected format: {format}")]
    Format { format: u32 },
    #[error("unexpected tn_len: {tn_len}")]
    TnLenRange { tn_len: i32 },
}

pub struct VersionlibData {
    pub version: [u32; 4],
    pub module_name: Result<String, std::string::FromUtf8Error>,
    pub module_name_raw: Vec<u8>,
    pub ptr_size: u32,
    pub data: hashbrown::HashMap<u64, u64>,
    pub rdata: hashbrown::HashMap<u64, u64>,
}

pub fn load(filename: &str) -> Result<VersionlibData, VersionlibError> {
    let mut f = File::open(filename).map_err(|e| VersionlibError::Open {
        reason: e.to_string(),
    })?;

    let format = f
        .read_u32::<LittleEndian>()
        .map_err(|e| VersionlibError::Read {
            reason: e.to_string(),
        })?;

    if format != 2 {
        return Err(VersionlibError::Format { format });
    }

    let version: [u32; 4] = (0..4)
        .map(|_| {
            f.read_u32::<LittleEndian>()
                .map_err(|e| VersionlibError::Read {
                    reason: e.to_string(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .unwrap();

    let tn_len = f
        .read_i32::<LittleEndian>()
        .map_err(|e| VersionlibError::Read {
            reason: e.to_string(),
        })?;

    if tn_len < 0 || tn_len >= 0x10000 {
        return Err(VersionlibError::TnLenRange { tn_len });
    }

    let mut module_name_raw = vec![0; tn_len as usize];
    f.read_exact(&mut module_name_raw)
        .map_err(|e| VersionlibError::Read {
            reason: e.to_string(),
        })?;
    let module_name = String::from_utf8(module_name_raw.clone());

    let ptr_size: u32 = f
        .read_u32::<LittleEndian>()
        .map_err(|e| VersionlibError::Read {
            reason: e.to_string(),
        })?;

    let addr_count: u32 = f
        .read_u32::<LittleEndian>()
        .map_err(|e| VersionlibError::Read {
            reason: e.to_string(),
        })?;

    let mut b1: u8;
    let mut b2: u8;
    let mut w1: u16;
    let mut w2: u16;
    let mut d1: u32;
    let mut d2: u32;
    let mut q1: u64;
    let mut q2: u64;

    let mut pvid: u64 = 0;
    let mut poffset: u64 = 0;
    let mut tpoffset: u64;

    let mut data = hashbrown::HashMap::<u64, u64>::with_capacity(addr_count as _);
    let mut rdata = hashbrown::HashMap::<u64, u64>::with_capacity(addr_count as _);

    for _ in 0..addr_count {
        let type_ = f.read_u8().map_err(|e| VersionlibError::Read {
            reason: e.to_string(),
        })?;
        let type_low = type_ & 0xF;
        let type_high = type_ >> 4;

        match type_low {
            0 => {
                q1 = f
                    .read_u64::<LittleEndian>()
                    .map_err(|e| VersionlibError::Read {
                        reason: e.to_string(),
                    })?
            }
            1 => q1 = pvid + 1,
            2 => {
                b1 = f.read_u8().map_err(|e| VersionlibError::Read {
                    reason: e.to_string(),
                })?;
                q1 = pvid + b1 as u64;
            }
            3 => {
                b1 = f.read_u8().map_err(|e| VersionlibError::Read {
                    reason: e.to_string(),
                })?;
                q1 = pvid - b1 as u64;
            }
            4 => {
                w1 = f
                    .read_u16::<LittleEndian>()
                    .map_err(|e| VersionlibError::Read {
                        reason: e.to_string(),
                    })?;
                q1 = pvid + w1 as u64;
            }
            5 => {
                w1 = f
                    .read_u16::<LittleEndian>()
                    .map_err(|e| VersionlibError::Read {
                        reason: e.to_string(),
                    })?;
                q1 = pvid - w1 as u64;
            }
            6 => {
                w1 = f
                    .read_u16::<LittleEndian>()
                    .map_err(|e| VersionlibError::Read {
                        reason: e.to_string(),
                    })?;
                q1 = w1 as _;
            }
            7 => {
                d1 = f
                    .read_u32::<LittleEndian>()
                    .map_err(|e| VersionlibError::Read {
                        reason: e.to_string(),
                    })?;
                q1 = d1 as _;
            }
            _ => unreachable!(),
        }

        tpoffset = if (type_high & 8) != 0 {
            poffset / ptr_size as u64
        } else {
            poffset
        };

        match type_high & 7 {
            0 => {
                q2 = f
                    .read_u64::<LittleEndian>()
                    .map_err(|e| VersionlibError::Read {
                        reason: e.to_string(),
                    })?
            }
            1 => q2 = tpoffset + 1,
            2 => {
                b2 = f.read_u8().map_err(|e| VersionlibError::Read {
                    reason: e.to_string(),
                })?;
                q2 = tpoffset + b2 as u64;
            }
            3 => {
                b2 = f.read_u8().map_err(|e| VersionlibError::Read {
                    reason: e.to_string(),
                })?;
                q2 = tpoffset - b2 as u64;
            }
            4 => {
                w2 = f
                    .read_u16::<LittleEndian>()
                    .map_err(|e| VersionlibError::Read {
                        reason: e.to_string(),
                    })?;
                q2 = tpoffset + w2 as u64;
            }
            5 => {
                w2 = f
                    .read_u16::<LittleEndian>()
                    .map_err(|e| VersionlibError::Read {
                        reason: e.to_string(),
                    })?;
                q2 = tpoffset - w2 as u64;
            }
            6 => {
                w2 = f
                    .read_u16::<LittleEndian>()
                    .map_err(|e| VersionlibError::Read {
                        reason: e.to_string(),
                    })?;
                q2 = w2 as _;
            }
            7 => {
                d2 = f
                    .read_u32::<LittleEndian>()
                    .map_err(|e| VersionlibError::Read {
                        reason: e.to_string(),
                    })?;
                q2 = d2 as _;
            }
            _ => unreachable!(),
        }

        if (type_high & 8) != 0 {
            q2 *= ptr_size as u64;
        }

        data.insert(q1, q2);
        rdata.insert(q2, q1);

        poffset = q2;
        pvid = q1;
    }

    Ok(VersionlibData {
        version,
        module_name,
        module_name_raw,
        ptr_size,
        data,
        rdata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = load("bin/versionlib-1-6-323-0.bin").unwrap();
        assert_eq!([1, 6, 323, 0], result.version);
        assert_eq!(401203, result.rdata[&0x2f9a800]);
        assert_eq!(51109, result.rdata[&0x8893c0]);
        assert_eq!(207886, result.rdata[&0x1753670]);
        assert_eq!(190143, result.rdata[&0x165dab0]);
        assert_eq!(14720, result.rdata[&0x1a1c00]);
        assert_eq!(14617, result.rdata[&0x19f080]);
        assert_eq!(195816, result.rdata[&0x1697a30]);
        assert_eq!(195890, result.rdata[&0x1699720]);
        assert_eq!(25259, result.rdata[&0x398f70]);
        assert_eq!(0x2f9a800, result.data[&401203]);
        assert_eq!(0x8893c0, result.data[&51109]);
        assert_eq!(0x1753670, result.data[&207886]);
        assert_eq!(0x165dab0, result.data[&190143]);
        assert_eq!(0x1a1c00, result.data[&14720]);
        assert_eq!(0x19f080, result.data[&14617]);
        assert_eq!(0x1697a30, result.data[&195816]);
        assert_eq!(0x1699720, result.data[&195890]);
        assert_eq!(0x398f70, result.data[&25259]);
    }
}
