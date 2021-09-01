use crate::binary::impls::{Magic, C, V3};
use crate::font::*;

use super::constants::*;
use super::impls::{Block, V1, V2};
use super::pack::{self, Unpack, UnpackDyn};

use std::io;

pub fn from_reader<R: io::Read>(mut reader: R) -> crate::Result<Font> {
    let mut vec = Vec::default();
    reader.read_to_end(&mut vec)?;
    from_bytes(vec.as_slice())
}

pub fn from_bytes(mut bytes: &[u8]) -> crate::Result<Font> {
    let magic: Magic = Unpack::<()>::unpack_take(&mut bytes)?;
    let mut builder = Assist::new(bytes, magic.version()?)?;
    builder.load()?;
    builder.build()
}

#[derive(Debug)]
struct Assist<'a> {
    src: &'a [u8],
    version: u8,
    info: Option<Info>,
    common: Option<Common>,
    pages: Vec<String>,
    chars: Vec<Char>,
    kernings: Vec<Kerning>,
}

impl<'a> Assist<'a> {
    fn new(src: &'a [u8], version: u8) -> crate::Result<Self> {
        if version == 3 {
            Ok(Self {
                src,
                version,
                info: None,
                common: None,
                pages: Vec::default(),
                chars: Vec::default(),
                kernings: Vec::default(),
            })
        } else {
            Err(crate::Error::UnsupportedBinaryVersion { version })
        }
    }

    fn build(mut self) -> crate::Result<Font> {
        let info = self.info.take().ok_or_else(|| crate::Error::NoInfoBlock)?;
        let common = self.common.take().ok_or_else(|| crate::Error::NoCommonBlock)?;
        Ok(Font::new(info, common, self.pages, self.chars, self.kernings))
    }

    fn load(&mut self) -> crate::Result<()> {
        while !self.src.is_empty() {
            self.next()?;
        }
        Ok(())
    }

    fn next(&mut self) -> crate::Result<()> {
        let (id, src) = self.block()?;
        match id {
            INFO => self.info(src),
            COMMON => self.common(src),
            PAGES => self.pages(src),
            CHARS => self.chars(src),
            KERNING_PAIRS => self.kerning_pairs(src),
            id => Err(crate::Error::InvalidBinaryBlock { id }),
        }
    }

    fn info(&mut self, src: &[u8]) -> crate::Result<()> {
        if self.info.is_some() {
            return Err(crate::Error::DuplicateInfoBlock { line: None });
        }
        self.info = Some(match self.version {
            2 | 3 => <Info as UnpackDyn<V2>>::unpack_dyn_tight(src)?,
            _ => unreachable!(),
        });
        Ok(())
    }

    fn common(&mut self, src: &[u8]) -> crate::Result<()> {
        if self.common.is_some() {
            return Err(crate::Error::DuplicateCommonBlock { line: None });
        }
        self.common = Some(match self.version {
            3 => <Common as Unpack<V3>>::unpack_tight(src)?,
            _ => unreachable!(),
        });
        Ok(())
    }

    fn pages(&mut self, src: &[u8]) -> crate::Result<()> {
        match self.version {
            1 | 2 | 3 => {
                <String as UnpackDyn<C>>::unpack_dyn_take_all(src, |file| {
                    self.pages.push(file);
                    Ok(())
                })?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    fn chars(&mut self, src: &[u8]) -> crate::Result<()> {
        match self.version {
            1 | 2 | 3 => {
                <Char as Unpack<V1>>::unpack_take_all(src, |char| {
                    self.chars.push(char);
                    Ok(())
                })?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    fn kerning_pairs(&mut self, src: &[u8]) -> crate::Result<()> {
        match self.version {
            1 | 2 | 3 => {
                <Kerning as Unpack<V1>>::unpack_take_all(src, |kerning| {
                    self.kernings.push(kerning);
                    Ok(())
                })?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    #[inline(always)]
    fn block(&mut self) -> crate::Result<(u8, &'a [u8])> {
        let Block { id, len } = match self.version {
            1 | 2 | 3 => <Block as Unpack>::unpack_take(&mut self.src)?,
            _ => unreachable!(),
        };
        Ok((id, self.bytes(len as usize)?))
    }

    #[inline(always)]
    fn bytes(&mut self, len: usize) -> crate::Result<&'a [u8]> {
        if len <= self.src.len() {
            let bytes = &self.src[..len];
            self.src = &self.src[len..];
            Ok(bytes)
        } else {
            pack::underflow()
        }
    }
}
