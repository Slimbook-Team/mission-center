<img align="left"  src="https://gitlab.com/mission-center-devs/mission-center/-/raw/main/data/icons/hicolor/scalable/apps/io.missioncenter.MissionCenter.svg" alt="drawing" width="64"/> 

# Mission Center

Monitor your CPU, Memory, Disk, Network and GPU usage with [Mission Center](https://missioncenter.io/)

![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0001-cpu-multi.png)

## Features

* Monitor overall or per-thread CPU usage
* See system process, thread, and handle count, uptime, clock speed (base and current), cache sizes
* Monitor RAM and Swap usage
* See a breakdown how the memory is being used by the system
* Monitor Disk utilization and transfer rates
* Monitor network utilization and transfer speeds
* See network interface information such as network card name, connection type (Wi-Fi or Ethernet), wireless speeds and
  frequency, hardware address, IP address
* Monitor overall GPU usage, video encoder and decoder usage, memory usage and power consumption, powered by the popular
  NVTOP project
* See a breakdown of resource usage by app and process
* Supports a minified summary view for simple monitoring
* Use OpenGL rendering for all the graphs in an effort to reduce CPU and overall resource usage
* Uses GTK4 and Libadwaita
* Written in Rust
* Flatpak first

## Limitations

Please note there is ongoing work to overcome all of these.

* No per-process network usage ([#3](https://gitlab.com/mission-center-devs/mission-center/-/issues/3))
* GPU support is experimental
* Intel GPU monitoring is only supported for Broadwell and later GPUs; and does not support VRAM, power, or temperature
  monitoring.
* When using Linux Mint/Cinnamon, launched applications may not show up in the "Applications" section. (Upstream
  issue: https://github.com/linuxmint/cinnamon/issues/12015)

Please also note that as Mission Center is a libadwaita application, it will not follow system-defined stylesheets (
themes).

## Installing

<a href="https://flathub.org/apps/io.missioncenter.MissionCenter"><img src="https://dl.flathub.org/assets/badges/flathub-badge-en.svg" width=200/></a>
<a href="https://gitlab.com/mission-center-devs/mission-center/-/jobs/7109267599/artifacts/raw/MissionCenter-x86_64.AppImage"><img src="https://raw.githubusercontent.com/AppImage/docs.appimage.org/master/source/_static/img/download-appimage-banner.svg" width=200/></a>

Also available from https://portable-linux-apps.github.io/apps/mission-center.html

Might also be available in your distribution's repository:  
[![](https://repology.org/badge/vertical-allrepos/mission-center.svg)](https://repology.org/project/mission-center/versions)

Source code is available at [GitLab](https://gitlab.com/mission-center-devs/mission-center)

## Screenshots

<details>
  <summary><b>Show</b></summary>

  <br/>

*CPU overall view*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0002-cpu-overall.png)

*Memory view*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0003-memory.png)

*Disk view*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0004-disk.png)

*Ethernet and Wi-Fi view*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0005-net-wired.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0006-net-wireless.png)

*GPU view*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0007-gpu-amd.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0008-gpu-nvidia.png)

*Apps page*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0009-apps.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0010-apps-filter.png)

*Services page*
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0018-services.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/019-services-filter.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0020-service-details.png)

*Dark mode*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0011-cpu-dark.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0012-disk-dark.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0013-gpu-nvidia-dark.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0014-apps-dark.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0021-services-dark.png)

*Summary view*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0015-cpu-summary-view.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0016-cpu-summary-view-dark.png)

  </details>

## Building and running

### Building - Native

**Requirements:**

* Meson (version >= 0.63)
* Rust (version >= 1.69)
* Python3
* Python GObject Introspection (required for Blueprint)
* DRM development libraries
* GBM development libraries
* udev development libraires
* GTK 4
* libadwaita

**Build instructions**

```bash
# On Ubuntu 24.04 all dependencies, except for the Rust toolchain, can be installed with:
sudo apt install build-essential curl git gettext python3-pip libadwaita-1-dev python3-gi libudev-dev libdrm-dev libgbm-dev desktop-file-utils meson libdbus-1-dev pkg-config

BUILD_ROOT="$(pwd)/build-meson-debug"

meson setup "$BUILD_ROOT" -Dbuildtype=debug # Alternatively pass `-Dbuildtype=release` for a release build
ninja -C "$BUILD_ROOT"
```

If you want to run the application from the build directory (for development or debugging) some set up is required:

```bash
export PATH="$BUILD_ROOT/src/sys_info_v2/gatherer:$PATH"
export GSETTINGS_SCHEMA_DIR="$BUILD_ROOT/data"
export HW_DB_DIR="$BUILD_ROOT/data/hwdb"
export MC_RESOURCE_DIR="$BUILD_ROOT/resources"

glib-compile-schemas --strict "$(pwd)/data" && mv "$(pwd)/data/gschemas.compiled" "$BUILD_ROOT/data/"
```

And then to run the app:

```bash
"$BUILD_ROOT/src/missioncenter"
```

If you want to install the app just run:

```bash
ninja -C _build install
```

And run the app from your launcher or from the command-line:

```bash
missioncenter
```

### Building - AppImage

```bash
# On Ubuntu 24.04 all dependencies, except for the Rust toolchain, can be installed with:
sudo apt install build-essential curl git gettext python3-pip libadwaita-1-dev python3-gi libudev-dev libdrm-dev libgbm-dev desktop-file-utils meson

meson setup _build -Dbuildtype=debug # Alternatively pass `-Dbuildtype=release` for a release build
ninja -C _build
```

And then build the AppImage:

```bash
meson install -C _build --no-rebuild --destdir "AppDir"

appimage-builder --appdir _build/AppDir/ 
```

And run the app from the command-line:

```bash
./"Mission Center-${version}-${arch}.AppImage"
```

### Building - Flatpak

**Requirements:**

* Flatpak
* Flatpak-Builder

Add the `flathub` repo is not already present:

```bash
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
```

Install the required Flatpak runtimes and SDKs:

```bash
flatpak install -y \
    org.freedesktop.Platform//24.08 \
    org.freedesktop.Sdk//24.08 \
    org.gnome.Platform//47 \
    org.gnome.Sdk//47
```

Finally build a Flatpak package:

```bash
cd flatpak
flatpak-builder --repo=repo --ccache --force-clean build io.missioncenter.MissionCenter.json
flatpak build-bundle repo missioncenter.flatpak io.missioncenter.MissionCenter
```

Install the package:

```bash
flatpak uninstall -y io.missioncenter.MissionCenter
flatpak install -y missioncenter.flatpak
```

Run the app from your launcher or from the command-line:

```bash
flatpak run io.missioncenter.MissionCenter
```

## Contributing

### Issues

Report issues to GitLab [issue tracking system](https://gitlab.com/mission-center-devs/mission-center/-/issues).

### Discord

Join [the Discord server](https://discord.gg/RG7QTeB9yk) and let's talk about what you think is missing or can be
improved.

### Translations

If you'd like to help translating Mission Center into your language, please head over
to [Weblate](https://hosted.weblate.org/engage/mission-center/).

<a href="https://hosted.weblate.org/engage/mission-center/">
  <img src="https://hosted.weblate.org/widgets/mission-center/-/mission-center/multi-auto.svg" alt="Translation status" />
</a>

Comments, suggestions, bug reports and contributions are welcome.

## License

This program is released under the terms of the GNU General Public License (GNU GPL) version 3. You can find a copy of
the license in the file COPYING.
