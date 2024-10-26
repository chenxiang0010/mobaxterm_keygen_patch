use anyhow::{anyhow, Result};
use console::style;
use dialogue_macro::Asker;
use regex::Regex;
use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    path::Path,
};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

const VARIANT_BASE64_TABLE: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";

#[derive(Asker, Debug)]
struct Config {
    #[input(prompt = "用户名", with_default = true)]
    username: String,
    #[input(prompt = "版本号")]
    version: String,
    #[select(
        prompt = "license类别", 
        options = ["Professional", "Educational", "Personal"],
        default = 0
  )]
    license_type: String,
    #[input(prompt = "license数量", default = 1)]
    count: usize,
    #[input(prompt = "MobaXterm安装路径")]
    install_path: String,
}

impl Config {
    fn new() -> Result<Config> {
        let config = Config::asker()
            .username(whoami::username())
            .version()
            .license_type()
            .count()
            .install_path()
            .finish();
        Ok(config)
    }
}

fn variant_base64_dict() -> HashMap<usize, char> {
    let mut dict = HashMap::new();
    for (index, val) in VARIANT_BASE64_TABLE.chars().enumerate() {
        dict.insert(index, val);
    }
    dict
}

fn parse_version(version: &str) -> Result<(&str, &str)> {
    let reg = Regex::new(r"^\d+\.\d+$")?;
    if reg.is_match(version) {
        let version_parts: Vec<&str> = version.split('.').collect();
        Ok((version_parts[0], version_parts[1]))
    } else {
        Err(anyhow!("Invalid version format"))
    }
}

fn parse_license_type(license_type: &str) -> u8 {
    match license_type {
        "Professional" => 1,
        "Educational" => 3,
        "Personal" => 4,
        _ => 1,
    }
}

fn encrypt_decrypt_bytes(key: &mut u16, bytes: &[u8], encrypt: bool) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(bytes.len());
    for &byte in bytes {
        result.push(byte ^ ((*key >> 8) as u8));
        *key = if encrypt {
            (*result.last().unwrap() as u16 & *key) | 0x482D
        } else {
            (byte as u16 & *key) | 0x482D
        };
    }
    result
}

fn variant_base64_encode(bytes: Vec<u8>) -> Vec<u8> {
    let base64_dict = variant_base64_dict();
    let mut result: Vec<u8> = Vec::new();
    let blocks_count = bytes.len() / 3;
    let leftover_bytes = bytes.len() % 3;

    for i in 0..blocks_count {
        let block = process_block_encode(i * 3, 3, &base64_dict, &bytes);
        result.extend_from_slice(&block);
    }

    if leftover_bytes > 0 {
        let block = process_block_encode(blocks_count * 3, leftover_bytes, &base64_dict, &bytes);
        result.extend_from_slice(&block);
    }
    result
}

fn process_block_encode(
    start_index: usize,
    byte_count: usize,
    base64_dict: &HashMap<usize, char>,
    bytes: &[u8],
) -> Vec<u8> {
    let coding_int = match byte_count {
        1 => i32::from_le_bytes([bytes[start_index], 0, 0, 0]),
        2 => i32::from_le_bytes([bytes[start_index], bytes[start_index + 1], 0, 0]),
        _ => i32::from_le_bytes([
            bytes[start_index],
            bytes[start_index + 1],
            bytes[start_index + 2],
            0,
        ]),
    };
    let step_count = match byte_count {
        3 => 4,
        1 => 2,
        _ => 3,
    };
    let mut block = String::new();
    for j in (0..(6 * step_count)).step_by(6) {
        block.push(base64_dict[&(((coding_int >> j) & 0x3F) as usize)]);
    }
    block.into_bytes()
}

fn build_license_code(config: &Config) -> Result<Vec<u8>> {
    let (major, minor) = parse_version(&config.version)?;
    let license_type = parse_license_type(&config.license_type);
    let license_string = format!(
        "{}#{}|{}{}#{}#{}3{}6{}#{}#{}#{}#",
        license_type, &config.username, major, minor, &config.count, major, minor, minor, 0, 0, 0
    );
    let encrypted_code = encrypt_decrypt_bytes(&mut 0x787, &license_string.into_bytes(), true);
    let license_code = variant_base64_encode(encrypted_code);
    Ok(license_code)
}

fn build_zip(license: &[u8], save_path: &str) -> Result<()> {
    let file_name = if !save_path.is_empty() && Path::new(save_path).exists() {
        Path::new(save_path).join("Custom.mxtpro")
    } else {
        Path::new("Custom.mxtpro").to_path_buf()
    };
    fs::write("Pro.key", license)?;
    let path = fs::File::create(file_name)?;
    let mut zip_file = ZipWriter::new(path);
    let options = FileOptions::<()>::default().compression_method(CompressionMethod::Stored);
    zip_file.start_file("Pro.key", options)?;

    let mut buffer = Vec::new();
    fs::File::open("Pro.key")?.read_to_end(&mut buffer)?;
    zip_file.write_all(&buffer)?;
    zip_file.finish()?;
    fs::remove_file("Pro.key")?;
    Ok(())
}

pub fn run() -> Result<()> {
    let config = Config::new()?;
    let license_code = build_license_code(&config)?;
    build_zip(&license_code, &config.install_path)?;
    println!("{}", style("生成成功，请打开MobaXterm查看!").green());
    Ok(())
}
