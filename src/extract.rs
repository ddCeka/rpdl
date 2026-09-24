use {
    crate::error::Result, flate2::read::GzDecoder, std::path::Path, tar::Archive, zip::ZipArchive,
};

pub async fn auto_extract(file_path: &Path, output_dir: &Path) -> Result<()> {
    let extension = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match extension {
        "zip" => extract_zip(file_path, output_dir).await?,
        "tar" | "gz" | "tgz" => extract_tar_gz(file_path, output_dir).await?,
        _ => return Ok(()),
    }

    println!("✓ Extracted: {}", file_path.display());
    Ok(())
}

async fn extract_zip(file_path: &Path, output_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(file_path)?;
    let mut archive = ZipArchive::new(file)?;
    archive.extract(output_dir)?;
    Ok(())
}

async fn extract_tar_gz(file_path: &Path, output_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(file_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive.unpack(output_dir)?;
    Ok(())
}
