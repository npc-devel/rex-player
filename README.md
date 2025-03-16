# Rex Player
Free media center / player

# Before compiling
Install rust toolchain: 
https://www.rust-lang.org/tools/install

# Install system libs  (Debian etc)
apt install -y mesa-common-dev libgl1-mesa-dev libglu1-mesa-dev nasm git libasound2-dev libavutil-dev libavformat-dev libavfilter-dev libavdevice-dev libxcb1-dev cmake ffmpeg

# Launching
target/<build>/rex-player (path-to-media-root)

# Command line options
-nsfw: Allow access to NSFW folder

# Nodes
Still in early stages to out of the box expects a folder structure like this:

<pre>
(media-root)\
  +--- Videos\
    +--- TV\
      +--- (show-x)\
        +--- Season 01 (optional - used for sequential play)\
    +--- Film\
    +--- XXX (nsfw)\
  +--- Music\
</pre>
Config for this is in the works - for now u can easily rearrage things by editing: src/script/common/rhai -- lines 395 to 426

You can also configure use of quad-media mode in this section.
  
  



