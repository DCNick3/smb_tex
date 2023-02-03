use std::path::PathBuf;
use clap::Parser;
use crate::texture::TexturePackage;

mod texture;

#[derive(clap::Parser, Debug)]
struct Cli {
    #[clap(subcommand)]
    command: CliCommand,
}

#[derive(clap::Subcommand, Debug)]
enum CliCommand {
    /// Extract textures from a tpg file
    Extract {
        /// Path to the tpg file
        path: PathBuf,
        /// Path to the output directory
        result: PathBuf,
    },
    /// Create a tpg file from a directory of textures
    Create {
        /// Path to the directory containing the textures
        path: PathBuf,
        /// Path to the output tpg file
        result: PathBuf,
        #[clap(long)]
        /// Change the used texture format
        force_format: Option<texture::TextureFormat>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        CliCommand::Extract { path, result } => {
            let data = std::fs::read(path).unwrap();
            let tp = texture::read_texture_package(&data).unwrap();

            std::fs::create_dir_all(&result).unwrap();

            for tex in tp.textures.iter() {
                let meta = &tex.meta;
                let path = result.join(format!("{:08x}.png", meta.id));
                tex.data.save(path).unwrap();
                std::fs::write(
                    result.join(format!("{:08x}.json", meta.id)),
                    serde_json::to_string_pretty(&meta).unwrap()
                ).unwrap()
            }
        }
        CliCommand::Create { path, result, force_format } => {
            let mut tp = TexturePackage::from_directory(&path).unwrap();
            if let Some(format) = force_format {
                for tex in tp.textures.iter_mut() {
                    tex.meta.texture_format = format;
                }
            }

            let data = texture::write_texture_package(&tp).unwrap();
            std::fs::write(result, data).unwrap();
        }
    }
}
