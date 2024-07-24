use std::{
    collections::HashMap,
    error::Error,
    fs,
    io::{self, Read, Write},
    path::Path,
};

use console::style;
use dialogue_macro::Asker;
use regex::Regex;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

#[derive(Asker, Debug)]
pub struct Config {
    #[input(prompt = "用户名")]
    username: String,
    #[input(prompt = "版本号")]
    version: String,
    #[select(
        prompt = "license类别", 
        options = ["Professional", "Educational", "Persional"],
        default = 0
    )]
    license_type: String,
    #[input(prompt = "license数量")]
    count: u8,
    #[input(prompt = "MobaXterm安装路径", default = "")]
    install_path: String,
}

impl Config {
    pub fn new() -> Result<Config, Box<dyn Error>> {
        let config = Config::asker()
            .username()
            .version()
            .license_type()
            .count()
            .install_path()
            .finish();
        Ok(config)
    }
}

fn variant_base64_dict() -> HashMap<usize, char> {
    let variant_base64_table = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
    let mut dict = HashMap::new();

    for (index, val) in variant_base64_table.chars().enumerate() {
        dict.insert(index, val);
    }
    dict
}

fn parse_version(version: &str) -> Result<(&str, &str), &str> {
    let reg = Regex::new(r"^\d+\.\d+$").unwrap();
    if reg.is_match(version) {
        let version_parts: Vec<&str> = version.split('.').collect();
        Ok((version_parts[0], version_parts[1]))
    } else {
        Err("Invalid version format")
    }
}

fn parse_license_type(license_type: &str) -> u8 {
    match license_type {
        "Professional" => 1,
        "Educational" => 3,
        "Persional" => 4,
        _ => 1,
    }
}

fn encrypt_decrypt_bytes(key: &mut u16, bs: &[u8], encrypt: bool) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(bs.len());
    for &b in bs {
        result.push(b ^ ((*key >> 8) as u8));
        *key = if encrypt {
            (*result.last().unwrap() as u16 & *key) | 0x482D
        } else {
            (b as u16 & *key) | 0x482D
        };
    }
    result
}

fn variant_base64_encode(bs: Vec<u8>) -> Vec<u8> {
    let base64_dict = variant_base64_dict();
    let mut result: Vec<u8> = Vec::new();
    let blocks_count = bs.len() / 3;
    let leftover_bytes = bs.len() % 3;

    for i in 0..blocks_count {
        let block = process_block_encode(i * 3, 3, &base64_dict, &bs);
        result.extend_from_slice(&block);
    }

    if leftover_bytes > 0 {
        let block = process_block_encode(blocks_count * 3, leftover_bytes, &base64_dict, &bs);
        result.extend_from_slice(&block);
    }

    result
}

fn process_block_encode(
    start_index: usize,
    byte_count: usize,
    base64_dict: &HashMap<usize, char>,
    bs: &[u8],
) -> Vec<u8> {
    let coding_int = match byte_count {
        1 => i32::from_le_bytes([bs[start_index], 0, 0, 0]),
        2 => i32::from_le_bytes([bs[start_index], bs[start_index + 1], 0, 0]),
        _ => i32::from_le_bytes([bs[start_index], bs[start_index + 1], bs[start_index + 2], 0]),
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

fn build_license_code(config: &Config) -> Result<Vec<u8>, Box<dyn Error>> {
    let (major, minor) = parse_version(&config.version)?;
    let license_type = parse_license_type(&config.license_type);
    let license_string = format!(
        "{}#{}|{}{}#{}#{}3{}6{}#{}#{}#{}#",
        license_type, &config.username, major, minor, &config.count, major, minor, minor, 0, 0, 0
    );
    let encrypt_code = encrypt_decrypt_bytes(&mut 0x787, &license_string.into_bytes(), true);
    let license_code = variant_base64_encode(encrypt_code);
    Ok(license_code)
}

fn build(license: &[u8], save_path: &str) -> io::Result<()> {
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

    // 将 Pro.key 文件的内容读入一个 buffer 中
    let mut buffer = Vec::new();
    fs::File::open("Pro.key")?.read_to_end(&mut buffer)?;

    // 将 buffer 中的内容写入 zip 压缩文件
    zip_file.write_all(&buffer)?;

    zip_file.finish()?; // 结束压缩文件的写入

    // 删除 "Pro.key" 文件
    fs::remove_file("Pro.key")?;
    Ok(())
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let license_code = build_license_code(&config)?;
    build(&license_code, &config.install_path)?;
    // 成功
    println!("{}", style("生成成功，请打开MobaXterm查看!").green());
    Ok(())
}
