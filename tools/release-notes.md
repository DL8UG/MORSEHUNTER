MORSEHUNTER is a CW contest simulator after Morse Runner 1.68 by VE3NEA, written new in Rust. You call CQ, stations answer, and you copy them by ear through noise, fading and interference. It takes the keyboard and a paddle, the Vail adapter included.

**Linux** (x86_64, glibc 2.35 or newer, so Ubuntu 22.04, Debian 12, Fedora 36 and later)

    tar xzf MORSEHUNTER-*-linux-x86_64.tar.gz
    cd MORSEHUNTER-*-linux-x86_64
    ./install.sh

This puts the program in `~/.local/bin` and an entry in the application menu. `./install.sh --remove` takes it away again. It needs ALSA (`libasound2`) and a graphics driver with Vulkan or OpenGL.

**Windows** (10 or newer, 64 bit)

Unpack the zip anywhere and start `morsehunter.exe`, there is nothing to install. The Windows build is new, reports of how it runs are welcome. The program has no code signing certificate, so SmartScreen warns on the first start. Click More info, then Run anyway.

**macOS** (11 or newer, Apple Silicon and Intel in one app)

Open the disk image and drag MORSEHUNTER to Applications. The app has no Apple developer signature, so macOS refuses the first start. Right-click it and choose Open. On macOS 15 confirm under System Settings, Privacy & Security, Open Anyway. Or run this once in the Terminal.

    xattr -dr com.apple.quarantine /Applications/MORSEHUNTER.app

**What it does**

- Pile-Up, Single calls, WPX and HST competition, as in the original
- keyboard keys as in Morse Runner, paddle on the Ctrl keys with its own keyer
- a Passthrough button for the Vail adapter, no `amidi` needed
- run codes to share the same run with others
- a score table for WPX, HST and skill points per hour
- the country of every logged station
- a waterfall above the filter, and five colour schemes, one of them high contrast
- window and manual in English, German, French, Spanish, Portuguese, Italian, Polish and Russian

Settings and scores of earlier builds (folder `morse-runner-rs`) move to the new folder `morsehunter` on the first start.
