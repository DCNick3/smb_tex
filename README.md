
# smb_tex 

Extract and pack textures for iOS version of Super Monkey Ball.

The format is actually pretty simple, it's just a bunch of headers and raw texels data. No compression, no mipmaps, no nothing. The only thing that makes it a bit tricky are different texture formats supported. 

It should be noted that texture names are not stored in the file, only the hash of the name. This means that the texture names are not human readable, but they can be found in the game's executable (I think).

# Usage

You can download pre-built binaries from the [releases page](https://github.com/DCNick3/smb_tex/releases) or build it yourself, for example with `cargo install --git https://github.com/DCNick3/smb_tex`.

Then you can use it like this:

```bash
smb_tex extract TexturePackage.tpg output_dir
```

The output directory will contain a .png file and a .json for each texture in the package. The .json file contains the texture's id, format, and other stuff I haven't really figured out.

You can modify the PNG files and then pack them back into a new package with:

```bash
smb_tex create output_dir new_TexturePackage.tpg
```

There is also an option to force the format of the textures, for example if you want to convert them to R8G8B8A8:

```bash
smb_tex create --force-format r8g8b8a8 output_dir new_TexturePackage.tpg
```