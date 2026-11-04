//! MacBinary files and resource forks (Inside Macintosh: More Macintosh
//! Toolbox, "Resource Manager").

use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a MacBinary file")]
    NotMacBinary,
    #[error("bad resource fork: {0}")]
    BadFork(&'static str),
    #[error("resource {0} {1} not found")]
    Missing(String, i16),
}

fn be16(d: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([d[o], d[o + 1]])
}

fn be32(d: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

/// The two forks of a MacBinary (I/II) file.
pub struct MacBinary {
    pub name: String,
    pub file_type: [u8; 4],
    pub creator: [u8; 4],
    pub data: Vec<u8>,
    pub rsrc: Vec<u8>,
}

impl MacBinary {
    pub fn parse(d: &[u8]) -> Result<MacBinary, Error> {
        if d.len() < 128 || d[0] != 0 || d[1] == 0 || d[1] > 63 || d[74] != 0 {
            return Err(Error::NotMacBinary);
        }
        let name = crate::text::mac_roman(&d[2..2 + d[1] as usize]);
        let dlen = be32(d, 83) as usize;
        let rlen = be32(d, 87) as usize;
        let dstart = 128;
        let rstart = dstart + dlen.div_ceil(128) * 128;
        if rstart + rlen > d.len() {
            return Err(Error::NotMacBinary);
        }
        Ok(MacBinary {
            name,
            file_type: d[65..69].try_into().unwrap(),
            creator: d[69..73].try_into().unwrap(),
            data: d[dstart..dstart + dlen].to_vec(),
            rsrc: d[rstart..rstart + rlen].to_vec(),
        })
    }
}

/// One resource.
pub struct Resource {
    pub name: Option<String>,
    pub data: Vec<u8>,
}

/// The resources of a fork, by (type, id).
pub struct Resources {
    pub map: BTreeMap<([u8; 4], i16), Resource>,
}

impl Resources {
    pub fn parse(f: &[u8]) -> Result<Resources, Error> {
        if f.len() < 16 {
            return Err(Error::BadFork("too short"));
        }
        let data_off = be32(f, 0) as usize;
        let map_off = be32(f, 4) as usize;
        let tl = map_off + be16(f, map_off + 24) as usize;
        let nl = map_off + be16(f, map_off + 26) as usize;
        let types = be16(f, tl).wrapping_add(1) as usize;
        let mut map = BTreeMap::new();
        for t in 0..types {
            let e = tl + 2 + t * 8;
            let ty: [u8; 4] = f[e..e + 4].try_into().unwrap();
            let count = be16(f, e + 4) as usize + 1;
            let refs = tl + be16(f, e + 6) as usize;
            for r in 0..count {
                let re = refs + r * 12;
                let id = be16(f, re) as i16;
                let name_off = be16(f, re + 2);
                let off = data_off + (be32(f, re + 4) & 0xff_ffff) as usize;
                let len = be32(f, off) as usize;
                if off + 4 + len > f.len() {
                    return Err(Error::BadFork("resource beyond the fork"));
                }
                let name = (name_off != 0xffff).then(|| {
                    let n = nl + name_off as usize;
                    crate::text::mac_roman(&f[n + 1..n + 1 + f[n] as usize])
                });
                map.insert((ty, id), Resource { name, data: f[off + 4..off + 4 + len].to_vec() });
            }
        }
        Ok(Resources { map })
    }

    pub fn get(&self, ty: &[u8; 4], id: i16) -> Result<&[u8], Error> {
        self.map
            .get(&(*ty, id))
            .map(|r| r.data.as_slice())
            .ok_or_else(|| Error::Missing(String::from_utf8_lossy(ty).into_owned(), id))
    }

    /// All ids of a type, ascending.
    pub fn ids(&self, ty: &[u8; 4]) -> Vec<i16> {
        self.map.keys().filter(|(t, _)| t == ty).map(|&(_, id)| id).collect()
    }
}

/// A resource fork holding one resource (no name, no attributes), as the
/// Resource Manager writes it: header, data, map.
pub fn single(ty: &[u8; 4], id: i16, data: &[u8]) -> Vec<u8> {
    let data_len = 4 + data.len() as u32;
    // map: header copy (16), handle (4), file ref (2), attributes (2),
    // type list and name list offsets (2 + 2), type count − 1 (2), one
    // type (8), one reference (12)
    let map_len = 16 + 4 + 2 + 2 + 2 + 2 + 2 + 8 + 12;
    let head = |f: &mut Vec<u8>| {
        for v in [256, 256 + data_len, data_len, map_len] {
            f.extend_from_slice(&v.to_be_bytes());
        }
    };
    let mut f = Vec::new();
    head(&mut f);
    f.resize(256, 0);
    f.extend_from_slice(&(data.len() as u32).to_be_bytes());
    f.extend_from_slice(data);
    head(&mut f);
    f.extend_from_slice(&[0; 8]);
    f.extend_from_slice(&28u16.to_be_bytes());
    f.extend_from_slice(&(map_len as u16).to_be_bytes());
    f.extend_from_slice(&0u16.to_be_bytes());
    f.extend_from_slice(ty);
    f.extend_from_slice(&0u16.to_be_bytes());
    f.extend_from_slice(&10u16.to_be_bytes());
    f.extend_from_slice(&id.to_be_bytes());
    f.extend_from_slice(&0xffffu16.to_be_bytes());
    f.extend_from_slice(&[0; 8]);
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_resource_fork_reads_back() {
        let f = single(b"HrSG", 0, &[1, 2, 3, 4, 5]);
        let r = Resources::parse(&f).unwrap();
        assert_eq!(r.get(b"HrSG", 0).unwrap(), &[1, 2, 3, 4, 5]);
        assert_eq!(r.ids(b"HrSG"), [0]);
        assert_eq!(r.map.len(), 1);
    }
}
