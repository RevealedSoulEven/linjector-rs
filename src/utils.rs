use hxdmp::hexdump;
use std::io::{ErrorKind, Read};

use crate::InjectionError;

const HEXDUMP_BUFFER_SIZE: usize = 0x200;
const TMP_DIR_PATH: &str = "/data/local/tmp";

pub fn print_file_hexdump(file_path: &str) -> Result<(), InjectionError> {
    let mut file = match std::fs::File::open(file_path) {
        Ok(file) => file,
        Err(e) => {
            error!("Error opening file: {}", e);
            return Err(InjectionError::FileError);
        }
    };

    let mut in_buffer = [0; HEXDUMP_BUFFER_SIZE];
    let mut out_buffer = Vec::new();

    match file.read_exact(&mut in_buffer) {
        Ok(_) => {}
        Err(e) => {
            if e.kind() == ErrorKind::UnexpectedEof {
                // ignore
            } else {
                error!("Error reading file: {}", e);
                return Err(InjectionError::FileError);
            }
        }
    }

    hexdump(&in_buffer, &mut out_buffer).unwrap();

    debug!("Hexdump of file: {}", String::from_utf8_lossy(&out_buffer));
    Ok(())
}

pub fn verify_elf_file(file_path: &str) -> Result<(), InjectionError> {
    let file = match std::fs::File::open(file_path) {
        Ok(file) => file,
        Err(e) => {
            error!("Error opening file: {}", e);
            return Err(InjectionError::FileError);
        }
    };

    let mut magic = [0; 4];
    match file.take(4).read_exact(&mut magic) {
        Ok(_) => {}
        Err(e) => {
            error!("Error reading file: {}", e);
            return Err(InjectionError::FileError);
        }
    }

    if magic != [0x7f, 0x45, 0x4c, 0x46] {
        error!("File is not an ELF file");
        return Err(InjectionError::FileError);
    }

    Ok(())
}

pub fn copy_file_to_tmp(file_path: &str) -> Result<String, InjectionError> {
    // get absolute path
    let file_path_absolute = match std::path::Path::new(file_path).canonicalize() {
        Ok(path) => path,
        Err(e) => {
            error!("Error getting file path: {}", e);
            return Err(InjectionError::FileError);
        }
    };

    info!("File path: {}", file_path_absolute.to_str().unwrap());

    // skip if the file is already in /dev/local/tmp
    if file_path_absolute.starts_with(TMP_DIR_PATH) {
        info!("File is already in {}", TMP_DIR_PATH);
        return Ok(String::from(file_path_absolute.to_str().unwrap()));
    }

    let file_name = match file_path_absolute.file_name() {
        Some(name) => name.to_str().unwrap(),
        None => {
            error!("Error getting file name");
            return Err(InjectionError::FileError);
        }
    };

    // copy file to /data/local/tmp so that the target app can access it
    let tmp_file_path = std::path::Path::new(TMP_DIR_PATH)
        .join(file_name)
        .as_os_str()
        .to_str()
        .unwrap()
        .to_string();

    info!("Copying file {} to {}", file_path, tmp_file_path);
    match std::fs::copy(file_path, &tmp_file_path) {
        Ok(_) => {
            info!("File copied successfully");
            Ok(tmp_file_path)
        }
        Err(e) => {
            error!("Error copying file: {}", e);
            Err(InjectionError::FileError)
        }
    }
}

pub fn fix_file_context(file_path: &str) -> Result<(), InjectionError> {
    // set file context to apk_data_file for dlopen to succeed
    info!("Fixing file context for {}", file_path);
    match std::process::Command::new("chcon")
        .arg("u:object_r:apk_data_file:s0")
        .arg(file_path)
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                error!(
                    "Error running chcon: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                Err(InjectionError::CommandError)
            } else {
                info!("File context fixed");
                Ok(())
            }
        }
        Err(e) => {
            error!("Error running chcon: {}", e);
            Err(InjectionError::CommandError)
        }
    }
}

pub fn fix_file_permissions(file_path: &str) -> Result<(), InjectionError> {
    // add executable permission to file
    info!("Fixing file permissions for {}", file_path);
    match std::process::Command::new("chmod")
        .arg("+r")
        .arg(file_path)
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                error!(
                    "Error running chmod: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                Err(InjectionError::CommandError)
            } else {
                info!("File permissions fixed");
                Ok(())
            }
        }
        Err(e) => {
            error!("Error running chmod: {}", e);
            Err(InjectionError::CommandError)
        }
    }
}
