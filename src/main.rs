use poe_bundle_reader::read_index;

fn main() {
    let data = std::fs::read("/Users/nihil/Downloads/index.bin").expect("Unable to read file");
    read_index(data.as_slice(), |index, paths| {
        println!("Got {} bundles",index.bundles.len());
        println!("Got {} files",index.files.len());
        println!("Got {} path_reps",index.path_reps.len());
        println!("Got {} filepaths", paths.len());
    });
}
