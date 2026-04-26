use zip::ZipArchive;
use std::fs::File;
use std::path::Path;

pub fn unzip<P: AsRef<Path>>(file_path: P, unzip_dir: P) -> Result<(), std::io::Error> {
    let file = File::open(file_path)?;
    let mut archive = ZipArchive::new(file)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = unzip_dir.as_ref().join(file.mangled_name());
        
        if file.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}