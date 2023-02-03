use std::path::PathBuf;
use clap::Parser;

mod texture;

#[derive(clap::Parser, Debug)]
struct Cli {
    path: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    walkdir::WalkDir::new(cli.path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().unwrap() == "tpg")
        .for_each(|e| {
            println!("{}", e.path().display());
            let data = std::fs::read(e.path()).unwrap();
            let tp = texture::read_texture_package(&data).unwrap();

            let dir = e.path().with_extension("");
            std::fs::create_dir_all(&dir).unwrap();
            for tex in tp.textures.iter() {
                let meta = tex.header.meta();
                let path = dir.join(format!("{:08x}.png", meta.id));
                tex.data.value.as_ref().unwrap().0.save(path).unwrap();
                std::fs::write(
                    dir.join(format!("{:08x}.json", meta.id)),
                    serde_json::to_string_pretty(&meta).unwrap()
                ).unwrap()
            }

            println!("{:#?}", tp);
        });
}
