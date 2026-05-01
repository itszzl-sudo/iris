fn main() {
    // 从 res/iris.png 生成 res/iris.ico（仅首次需要）
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let ico_path = std::path::Path::new(&manifest_dir).join("res/iris.ico");
    let png_path = std::path::Path::new(&manifest_dir).join("res/iris.png");

    if !ico_path.exists() {
        let png_data = std::fs::read(&png_path).expect("res/iris.png not found");
        let ico_data = png_to_ico(&png_data);
        std::fs::write(&ico_path, ico_data).expect("failed to write res/iris.ico");
        println!("cargo:warning=Generated res/iris.ico from iris.png");
    }

    embed_resource::compile(
        std::path::Path::new(&manifest_dir).join("res/icon.rc"),
        embed_resource::NONE,
    );
}

/// 将 PNG 数据包装为 ICO 格式（ICO v3 直接存储 PNG 字节流）
fn png_to_ico(png: &[u8]) -> Vec<u8> {
    // 从 PNG IHDR 块读取尺寸（固定偏移 16/20）
    let w = u16::from_be_bytes([png[16], png[17]]);
    let h = u16::from_be_bytes([png[20], png[21]]);

    let size = png.len() as u32;
    let offset: u32 = 6 + 16; // ICO 头部 + 1 个目录条目

    let mut ico = Vec::with_capacity(offset as usize + png.len());

    // ICO 头部: reserved(2)=0, type(2)=1(icon), count(2)=1
    ico.extend_from_slice(&[0x00, 0x00, 0x01, 0x00, 0x01, 0x00]);

    // 目录条目
    ico.push(if w >= 256 { 0 } else { w as u8 });
    ico.push(if h >= 256 { 0 } else { h as u8 });
    ico.push(0); // 调色板颜色数 (0 = 无)
    ico.push(0); // 保留
    ico.extend_from_slice(&1u16.to_le_bytes()); // 颜色平面
    ico.extend_from_slice(&32u16.to_le_bytes()); // 每像素位数
    ico.extend_from_slice(&size.to_le_bytes()); // 图像数据大小
    ico.extend_from_slice(&offset.to_le_bytes()); // 图像数据偏移

    // PNG 字节（ICO v3 直接存储）
    ico.extend_from_slice(png);

    ico
}
