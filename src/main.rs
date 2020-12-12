use poe_bundle::reader::BundleReader;
use poe_bundle::reader::BundleReaderRead;

fn main() {
    let reader = BundleReader::from_install("/Users/nihil/code/poe-files");
    let size = reader.size_of("Data/Mods.dat").unwrap();

    let mut dst = Vec::with_capacity(size);
    reader.write_into("Data/Mods.dat", &mut dst).unwrap();
    println!("got mods data {}", dst.len())
}