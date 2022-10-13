use poe_bundle::reader::BundleReader;
use poe_bundle::reader::BundleReaderRead;

fn main() {
    let reader = BundleReader::from_install(r#"/home/nihil/Games/path-of-exile/drive_c/Program Files (x86)/Grinding Gear Games/Path of Exile"#);
    let size = reader.size_of("Data/Mods.dat").unwrap();

    let mut dst = Vec::with_capacity(size);
    reader.write_into("Data/Mods.dat", &mut dst).unwrap();
    println!("got mods data {}", dst.len())
}