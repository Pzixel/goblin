/// Demonstrates the magical powers of scroll + goblin
/// Goblin implements `TryFromCtx` for the header type
/// which means downstream crates/clients can just "parse" headers out of
/// arbitrary buffers, without learning new crate specific function names
/// I.e., all you need are Types + Pread = Happiness

extern crate scroll;
extern crate goblin;
extern crate failure;

use failure::{bail, err_msg, Error};
use goblin::container::Endian;
use goblin::pe::data_directories::DataDirectory;
use goblin::pe::PE;
use scroll::ctx::TryFromCtx;
use scroll::Pread;
use goblin::pe::utils::find_offset;

#[repr(C)]
#[derive(Debug, Pread)]
pub struct CliHeader {
    pub cb: u32,
    pub major_version: u16,
    pub minor_version: u16,
    pub metadata: DataDirectory,
    pub flags: u32,
    pub entry_point_token: u32,
}

#[repr(C)]
#[derive(Debug)]
struct MetadataRoot<'a> {
    pub signature: u32,
    pub major_version: u16,
    pub minor_version: u16,
    _reserved: u32,
    pub length: u32,
    pub version: &'a str,
}

impl<'a> TryFromCtx<'a, Endian> for MetadataRoot<'a> {
    type Error = scroll::Error;
    type Size = usize;
    
    fn try_from_ctx(src: &'a [u8], endian: Endian) -> Result<(Self, Self::Size), Self::Error> {
        let offset = &mut 0;
        let signature = src.gread_with(offset, endian)?;
        let major_version = src.gread_with(offset, endian)?;
        let minor_version = src.gread_with(offset, endian)?;
        let reserved = src.gread_with(offset, endian)?;
        let length = src.gread_with(offset, endian)?;
        let version = src.gread(offset)?;
        Ok((
            Self {
                signature,
                major_version,
                minor_version,
                _reserved: reserved,
                length,
                version,
            },
            *offset,
        ))
    }
}

fn main() -> Result<(), Error> {
    let file = include_bytes!("../assets/dotnet_executable_example.dll");
    let file = &file[..];
    let pe = PE::parse(file)?;
    if pe.header.coff_header.machine != 0x14c {
        bail!("Is not a .Net executable");
    }
    let optional_header = pe.header.optional_header.ok_or_else(|| err_msg("No optional header"))?;
    let file_alignment = optional_header.windows_fields.file_alignment;
    let cli_header = optional_header
        .data_directories
        .get_clr_runtime_header()
        .ok_or_else(|| err_msg("No CLI header"))?;
    let sections = &pe.sections;

    let rva = cli_header.virtual_address as usize;
    let offset = find_offset(rva, sections, file_alignment).ok_or_else(|| err_msg("Cannot map rva into offset"))?;
    let cli_header_value: CliHeader = file.pread_with(offset, scroll::LE)?;

    println!("{:#?}", cli_header_value);
    let rva = cli_header_value.metadata.virtual_address as usize;
    let offset = find_offset(rva, sections, file_alignment).ok_or_else(|| err_msg("Cannot map rva into offset"))?;
    let root: MetadataRoot = file.pread_with(offset, scroll::LE)?;
    println!("{:#?}", root);

    Ok(())
}
