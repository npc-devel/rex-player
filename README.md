# Rex Player
Free media center / player

# Before compiling
Install rust toolchain: 
https://www.rust-lang.org/tools/install

# Install system libs  (Debian etc)
apt install -y mesa-common-dev libgl1-mesa-dev libglu1-mesa-dev nasm git libasound2-dev libavutil-dev libavformat-dev libavfilter-dev libavdevice-dev libxcb1-dev cmake ffmpeg

# Pipewire users:
apt install pipewire-alsa

# Launching
(project root)\/target\/(build)\/rex-player (path-to-media-root-including-trailing-slash)

# Command line options
-nsfw: Allow access to NSFW folder

# Expected folder structure (config coming soon)
Still in early stages so out of the box expects a folder structure like this:

<pre>
(media-root)
  +--- Videos
    +--- TV
      +--- (show-x)
        +--- Season 01 (optional - used for sequential play)
    +--- Film
    +--- XXX (nsfw)
  +--- Music
</pre>

Config for this is in the works - for now you can easily rearrage things by editing: src/script/common/rhai -- lines 395 to 426

You can also configure use of quad-media mode in this section.
  
# 3rd party software used
Video & audio playback:
https://www.ffmpeg.org

Audio visualisation:
https://github.com/projectM-visualizer/projectm


# Support 
Support requests please email me at: npc.dev.github@gmail.com ... i'll get back to you as soon as possible! :) 

