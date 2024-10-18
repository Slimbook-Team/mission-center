#!/bin/bash

# shellcheck disable=SC2164

set -e

if [[ -z "$SRC_PATH" ]]; then
    echo "SRC_PATH is not set or empty"
    exit 1
fi

if [[ -z "$OUT_PATH" ]]; then
    echo "OUT_PATH is not set or empty"
    exit 1
fi

export HOME=/root
export TERM=xterm
export PATH="/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin:/bin:/sbin:/usr/lib/gcc/$(arch)-linux-gnu/9:/usr/lib/gcc/$(arch)-linux-gnu/11"
export LD_LIBRARY_PATH="/usr/lib/gcc/$(arch)-linux-gnu/9:/usr/lib/gcc/$(arch)-linux-gnu/11"

apt-get update

ln -sf /usr/share/zoneinfo/Etc/UTC /etc/localtime
DEBIAN_FRONTEND=noninteractive apt-get install -y tzdata
dpkg-reconfigure --frontend noninteractive tzdata

apt-get install -y curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain=1.80.1 -y

apt-get install -y build-essential flex bison git gettext python3-pip python3-gi libudev-dev libdrm-dev libgbm-dev libdbus-1-dev libxslt-dev libpcre2-dev libfuse3-dev libgcrypt-dev libjpeg-turbo8-dev libpng-dev libisocodes-dev libepoxy-dev libxrandr-dev libxi-dev libxcursor-dev libxdamage-dev libxinerama-dev libgstreamer-plugins-bad1.0-dev libpixman-1-dev libfontconfig1-dev libxkbcommon-dev libcurl4-openssl-dev libyaml-dev libzstd-dev libgraphviz-dev librsvg2-2 libtiff5 libbrotli-dev shared-mime-info desktop-file-utils pkg-config gperf itstool xsltproc valac docbook-xsl libxml2-utils python3-packaging
ln -sf python3 /usr/bin/python
pip3 install cmake meson ninja

mkdir -p $OUT_PATH && cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/fribidi.html
# ----------------------------------------------------------------------
FRIBIDI_VER=1.0.15
# ----------------------------------------------------------------------
curl -LO https://github.com/fribidi/fribidi/releases/download/v$FRIBIDI_VER/fribidi-$FRIBIDI_VER.tar.xz
tar xvf fribidi-$FRIBIDI_VER.tar.xz
cd fribidi-$FRIBIDI_VER
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf fribidi-$FRIBIDI_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/glib2.html
# --------------------------------------------------------------------
GLIB_VER=2.81.1
GLIB_VER_MM=$(echo $GLIB_VER | cut -f1-2 -d'.')
# --------------------------------------------------------------------
rm -rf /usr/include/glib-2.0/
curl -LO https://download.gnome.org/sources/glib/$GLIB_VER_MM/glib-$GLIB_VER.tar.xz
tar xvf glib-$GLIB_VER.tar.xz
cd glib-$GLIB_VER
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                \
    -Dselinux=disabled                 \
    -Dman-pages=disabled
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf glib-$GLIB_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/gobject-introspection.html
# ------------------------------------------------------------------------------------
GOBJ_INTRSPEC_VER=1.80.1
GOBJ_INTRSPEC_VER_MM=$(echo $GOBJ_INTRSPEC_VER | cut -f1-2 -d'.')
# ------------------------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/gobject-introspection/$GOBJ_INTRSPEC_VER_MM/gobject-introspection-$GOBJ_INTRSPEC_VER.tar.xz
tar xvf gobject-introspection-$GOBJ_INTRSPEC_VER.tar.xz
cd gobject-introspection-$GOBJ_INTRSPEC_VER
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf gobject-introspection-$GOBJ_INTRSPEC_VER*
cd $OUT_PATH

# Yes, compile it again because I think there is a circular dependency with `gobject-introspection`
# https://www.linuxfromscratch.org/blfs/view/stable/general/glib2.html
# --------------------------------------------------------------------
GLIB_VER=$GLIB_VER
GLIB_VER_MM=$GLIB_VER_MM
# --------------------------------------------------------------------
rm -rf /usr/include/glib-2.0/
curl -LO https://download.gnome.org/sources/glib/$GLIB_VER_MM/glib-$GLIB_VER.tar.xz
tar xvf glib-$GLIB_VER.tar.xz
cd glib-$GLIB_VER
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                \
    -Dselinux=disabled                 \
    -Dman-pages=disabled
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf glib-$GLIB_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/freetype2.html
# -------------------------------------------------------------------
FREETYPE_VER=2.13.3
# -------------------------------------------------------------------
curl -LO https://downloads.sourceforge.net/freetype/freetype-$FREETYPE_VER.tar.xz
tar xvf freetype-$FREETYPE_VER.tar.xz
cd freetype-$FREETYPE_VER
sed -ri "s:.*(AUX_MODULES.*valid):\1:" modules.cfg
sed -r "s:.*(#.*SUBPIXEL_RENDERING) .*:\1:" -i include/freetype/config/ftoption.h
CFLAGS=-O2 ./configure                    \
    --prefix=/usr                         \
    --libdir=/usr/lib/$(arch)-linux-gnu   \
    --enable-freetype-config              \
    --without-harfbuzz                    \
    --disable-static
make -j4
make install && make DESTDIR=$OUT_PATH install
cd ../ && rm -rf freetype-$FREETYPE_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/gdk-pixbuf.html
# -------------------------------------------------------------------
GDK_PIXBUF_VER=2.42.12
GDK_PIXBUF_VER_MM=$(echo $GDK_PIXBUF_VER | cut -f1-2 -d'.')
# -------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/gdk-pixbuf/$GDK_PIXBUF_VER_MM/gdk-pixbuf-$GDK_PIXBUF_VER.tar.xz
tar xvf gdk-pixbuf-$GDK_PIXBUF_VER.tar.xz
cd gdk-pixbuf-$GDK_PIXBUF_VER
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                \
    -Dman=false                        \
    --wrap-mode=nofallback
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../.. && rm -rf gdk-pixbuf-$GDK_PIXBUF_VER*
cd $OUT_PATH

# Pixman
# -------------------------------------------------------------------
PIXMAN_VER=e7ef051a6d9edd2d8bd84fe2fda3ca54ba83d1c8
# -------------------------------------------------------------------
curl -LO https://gitlab.freedesktop.org/pixman/pixman/-/archive/$PIXMAN_VER/pixman-$PIXMAN_VER.tar.bz2
tar xvf pixman-$PIXMAN_VER.tar.bz2
cd pixman-$PIXMAN_VER
mkdir build && cd build
/usr/local/bin/meson setup ..           \
    --prefix=/usr                       \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                 \
    -Dtests=disabled                    \
    -Ddemos=disabled                    \
    -Dgtk=disabled
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf pixman-$PIXMAN_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/graphene.html
# -----------------------------------------------------------------
GRAPHENE_VER=1.10.8
GRAPHENE_VER_MM=$(echo $GRAPHENE_VER | cut -f1-2 -d'.')
# -----------------------------------------------------------------
curl -LO https://download.gnome.org/sources/graphene/$GRAPHENE_VER_MM/graphene-$GRAPHENE_VER.tar.xz
tar xvf graphene-$GRAPHENE_VER.tar.xz
cd graphene-$GRAPHENE_VER
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../.. && rm -rf graphene-$GRAPHENE_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/cairo.html
# --------------------------------------------------------------
CAIRO_VER=1.18.2
# --------------------------------------------------------------
curl -LO https://gitlab.freedesktop.org/cairo/cairo/-/archive/$CAIRO_VER/cairo-$CAIRO_VER.tar.bz2
tar xvf cairo-$CAIRO_VER.tar.bz2
cd cairo-$CAIRO_VER
sed -e "/@prefix@/a exec_prefix=@exec_prefix@" -i util/cairo-script/cairo-script-interpreter.pc.in
mkdir build && cd build
/usr/local/bin/meson setup --wipe       \
    --prefix=/usr                       \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                 \
    -Dtests=disabled                    \
    -Dtee=disabled                      \
    -Dxcb=disabled                      \
    -Dxlib-xcb=enabled                  \
    -Dpng=enabled                       \
    -Dzlib=enabled
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../.. && rm -rf cairo-$CAIRO_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/python-modules.html#pycairo
# -------------------------------------------------------------------------------------
PYCAIRO_VER=1.26.1
# --------------------------------------------------------------
curl -LO https://github.com/pygobject/pycairo/releases/download/v$PYCAIRO_VER/pycairo-$PYCAIRO_VER.tar.gz
tar xvf pycairo-$PYCAIRO_VER.tar.gz
cd pycairo-$PYCAIRO_VER
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../.. && rm -rf pycairo-$PYCAIRO_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/python-modules.html#pygobject3
# ----------------------------------------------------------------------------------------
PYGOBJ_VER=3.48.2
PYGOBJ_VER_MM=$(echo $PYGOBJ_VER | cut -f1-2 -d'.')
# ----------------------------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/pygobject/$PYGOBJ_VER_MM/pygobject-$PYGOBJ_VER.tar.xz
tar xvf pygobject-$PYGOBJ_VER.tar.xz
cd pygobject-$PYGOBJ_VER
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../.. && rm -rf pygobject-$PYGOBJ_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/wayland.html
# ----------------------------------------------------------------------
WAYLAND_VER_REL=1.23.0-1
WAYLAND_VER=$(echo $WAYLAND_VER_REL | cut -f1 -d'-')
# ----------------------------------------------------------------------
curl -LO https://launchpad.net/ubuntu/+archive/primary/+sourcefiles/wayland/$WAYLAND_VER_REL/wayland_$WAYLAND_VER.orig.tar.gz
tar xvf wayland_$WAYLAND_VER.orig.tar.gz
cd wayland-$WAYLAND_VER
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release -Ddocumentation=false ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf wayland*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/wayland-protocols.html
# --------------------------------------------------------------------------------
WAYLAND_PROTO_VER_REL=1.36-1
WAYLAND_PROTO_VER=$(echo $WAYLAND_PROTO_VER_REL | cut -f1 -d'-')
# ----------------------------------------------------------------------
curl -LO https://launchpad.net/ubuntu/+archive/primary/+sourcefiles/wayland-protocols/$WAYLAND_PROTO_VER_REL/wayland-protocols_$WAYLAND_PROTO_VER.orig.tar.xz
tar xvf wayland-protocols_$WAYLAND_PROTO_VER.orig.tar.xz
cd wayland-protocols-$WAYLAND_PROTO_VER
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf wayland-protocols*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/adwaita-icon-theme.html
# ---------------------------------------------------------------------------
ADW_ICONS_VER=47.0
ADW_ICONS_VER_MM=$(echo $ADW_ICONS_VER | cut -f1 -d'.')
# ---------------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/adwaita-icon-theme/$ADW_ICONS_VER_MM/adwaita-icon-theme-$ADW_ICONS_VER.tar.xz
tar xvf adwaita-icon-theme-$ADW_ICONS_VER.tar.xz
cd adwaita-icon-theme-$ADW_ICONS_VER
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf adwaita-icon-theme-$ADW_ICONS_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/harfbuzz.html
# -----------------------------------------------------------------------
HARFBUZZ_VER=9.0.0
# -----------------------------------------------------------------------
curl -LO https://github.com/harfbuzz/harfbuzz/releases/download/$HARFBUZZ_VER/harfbuzz-$HARFBUZZ_VER.tar.xz
tar xvf harfbuzz-$HARFBUZZ_VER.tar.xz
cd harfbuzz-$HARFBUZZ_VER
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                \
    -Dgraphite2=disabled               \
    -Dtests=disabled
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd $OUT_PATH && rm -rf harfbuzz-$HARFBUZZ_VER*
# FreeType and Harfbuzz depend on each other, so we need to compile FreeType again
curl -LO https://downloads.sourceforge.net/freetype/freetype-$FREETYPE_VER.tar.xz
tar xvf freetype-$FREETYPE_VER.tar.xz
cd freetype-$FREETYPE_VER
sed -ri "s:.*(AUX_MODULES.*valid):\1:" modules.cfg
sed -r "s:.*(#.*SUBPIXEL_RENDERING) .*:\1:" -i include/freetype/config/ftoption.h
CFLAGS=-O2 ./configure                    \
    --prefix=/usr                         \
    --libdir=/usr/lib/$(arch)-linux-gnu   \
    --enable-freetype-config              \
    --disable-static
make -j4
make install && make DESTDIR=$OUT_PATH install
cd ../ && rm -rf freetype-$FREETYPE_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/pango.html
# ------------------------------------------------------------------
PANGO_VER=1.54.0
PANGO_VER_MM=$(echo $PANGO_VER | cut -f1-2 -d'.')
# ------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/pango/$PANGO_VER_MM/pango-$PANGO_VER.tar.xz
tar xvf pango-$PANGO_VER.tar.xz
cd pango-$PANGO_VER
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf pango-$PANGO_VER*
cd $OUT_PATH

# GLSLC
# -------------------------------------------------------------
GLSLC_VER=2024.1
# -------------------------------------------------------------
curl -LO https://github.com/google/shaderc/archive/refs/tags/v$GLSLC_VER.tar.gz
tar xvf v$GLSLC_VER.tar.gz
cd shaderc-$GLSLC_VER
./utils/git-sync-deps
mkdir build && cd build
cmake -GNinja -S ..                    \
    -DCMAKE_INSTALL_PREFIX=/usr        \
    -DCMAKE_BUILD_TYPE=Release         \
    -DSHADERC_SKIP_INSTALL=OFF         \
    -DSHADERC_SKIP_TESTS=ON            \
    -DSHADERC_SKIP_EXAMPLES=ON         \
    -DSHADERC_SKIP_COPYRIGHT_CHECK=ON  \
    -DSHADERC_ENABLE_WERROR_COMPILE=OFF
ninja && ninja install
cd ../../ && rm -rf v$GLSLC_VER* shaderc-$GLSLC_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/gtk4.html
# -------------------------------------------------------------
GTK_VER=4.16.2
GTK_VER_MM=$(echo $GTK_VER | cut -f1-2 -d'.')
# -------------------------------------------------------------
curl -LO https://download.gnome.org/sources/gtk/$GTK_VER_MM/gtk-$GTK_VER.tar.xz
tar xvf gtk-$GTK_VER.tar.xz
cd gtk-$GTK_VER
# Patch for GLAD Vulkan support
for f in $SRC_PATH/support/patches/gtk4/*.patch; do
  patch -p1 < $f
done
# Copy glad project into GTK source directory
cp -rv $SRC_PATH/support/patches/gtk4/glad .
mkdir build && cd build
/usr/local/bin/meson setup ..           \
    --prefix=/usr                       \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                 \
    -Dvulkan=enabled                    \
    -Dintrospection=enabled             \
    -Dbuild-examples=false              \
    -Dbuild-tests=false                 \
    -Dbuild-demos=false                 \
    -Dbuild-testsuite=false             \
    -Dbroadway-backend=false            \
    -Dmedia-gstreamer=disabled          \
    -Dprint-cups=disabled
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf gtk-$GTK_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/vala.html
# -------------------------------------------------------------------
VALA_VER=0.56.17
VALA_VER_MM=$(echo $VALA_VER | cut -f1-2 -d'.')
# -------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/vala/$VALA_VER_MM/vala-$VALA_VER.tar.xz
tar xvf vala-$VALA_VER.tar.xz
cd vala-$VALA_VER
CFLAGS=-O2 ./configure --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu
make -j4 && make install && make DESTDIR=$OUT_PATH install
cd ../ && rm -rf vala-$VALA_VER*
cd $OUT_PATH

# AppStream
# -------------------------------------------------------------------
APPSTREAM_VER=1.0.3
# -------------------------------------------------------------------
curl -LO https://www.freedesktop.org/software/appstream/releases/AppStream-$APPSTREAM_VER.tar.xz
tar xvf AppStream-$APPSTREAM_VER.tar.xz
cd AppStream-$APPSTREAM_VER
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                \
    -Dstemming=false                   \
    -Dsystemd=false                    \
    -Dvapi=false                       \
    -Dapidocs=false                    \
    -Dinstall-docs=false
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf AppStream-$APPSTREAM_VER*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/libadwaita.html
# -------------------------------------------------------------------
LIBADW_VER=1.6.0
LIBADW_VER_MM=$(echo $LIBADW_VER | cut -f1-2 -d'.')
# -------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/libadwaita/$LIBADW_VER_MM/libadwaita-$LIBADW_VER.tar.xz
tar xvf libadwaita-$LIBADW_VER.tar.xz
cd libadwaita-$LIBADW_VER
# Patch for Yaru support
for f in $SRC_PATH/support/patches/libadwaita/*.patch; do
  patch -p1 < $f
done
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                \
    -Dtests=false                      \
    -Dexamples=false
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf libadwaita-$LIBADW_VER*
cd $OUT_PATH

# Blueprint Compiler
# -------------------------------------------------------------------
BP_CMP_VER=b308adc3af939b27bd18941710b4499b1131e090
# -------------------------------------------------------------------
curl -LO https://gitlab.gnome.org/jwestman/blueprint-compiler/-/archive/$BP_CMP_VER/blueprint-compiler-$BP_CMP_VER.tar.bz2
tar xvf blueprint-compiler-*.tar.bz2 && rm blueprint-compiler-*.tar.bz2
cd blueprint-compiler-*
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release
ninja && ninja install
cd ../../ && rm -rf blueprint-compiler-*
# Patch for compatibility with Python 3.8
sed -i '1s/^/from __future__ import annotations\n/' /usr/lib/python3/dist-packages/blueprintcompiler/gir.py
sed -i '1s/^/from __future__ import annotations\n/' /usr/lib/python3/dist-packages/blueprintcompiler/ast_utils.py
sed -i '1s/^/from __future__ import annotations\n/' /usr/lib/python3/dist-packages/blueprintcompiler/decompiler.py

cd $OUT_PATH
rm -rf $OUT_PATH/usr/bin

export PATH="$HOME/.cargo/bin:$PATH"
cargo install toml2json

cd $SRC_PATH
BUILD_DIR=_build-$(arch)
rm -rf $BUILD_DIR && meson setup $BUILD_DIR -Dbuildtype=release -Dprefix=/usr
ninja -C $BUILD_DIR && env DESTDIR=$OUT_PATH ninja -C $BUILD_DIR install

glib-compile-schemas $OUT_PATH/usr/share/glib-2.0/schemas/

cd $OUT_PATH
rm -rfv usr/include/ usr/lib/{python3,$(arch)-linux-gnu/{*.la,cairo/*.la,cmake,girepository-1.0,glib-2.0,gobject-introspection,graphene-1.0,pkgconfig,libvala*,vala-*,valadoc-*,libgirepository*,libsass.so,libharfbuzz-{cairo*,gobject*,icu*},libwayland-cursor*,libwayland-server*}} usr/libexec/ usr/share/{aclocal,appstream,bash-completion,devhelp,gdb,gettext,glib-2.0/{codegen,dtds,gdb,gettext,valgrind},gobject-introspection-1.0,gtk-4.0/valgrind,gtk-doc,installed-tests,man,pkgconfig,thumbnailers,vala,vala-*,valadoc-*,wayland,wayland-protocols}
cp -Lv /usr/lib/$(arch)-linux-gnu/{libffi.so.7,libjpeg.so.8,libtiff.so.5,libpng16.so.16,libX11.so.6,libXcursor.so.1,libXdamage.so.1,libXext.so.6,libXfixes.so.3,libXi.so.6,libXinerama.so.1,libXrandr.so.2,libXrender.so.1,libxkbcommon.so.0,libepoxy.so.0,libfontconfig.so.1,libcurl.so.4,libnghttp2.so.14,libidn2.so.0,librtmp.so.1,libssh.so.4,libpsl.so.5,libssl.so.1.1,libcrypt.so.1,libcrypto.so.1.1,libgssapi_krb5.so.2,libldap_r-2.4.so.2,liblber-2.4.so.2,libbrotlidec.so.1,libunistring.so.2,libgnutls.so.30,libhogweed.so.5,libnettle.so.7,libgmp.so.10,libkrb5.so.3,libk5crypto.so.3,libkrb5support.so.0,libsasl2.so.2,libgssapi.so.3,libbrotlicommon.so.1,libp11-kit.so.0,libtasn1.so.6,libheimntlm.so.0,libkrb5.so.26,libasn1.so.8,libhcrypto.so.4,libroken.so.18,libwind.so.0,libheimbase.so.1,libhx509.so.5,libsqlite3.so.0,libxml2.so.2,libxmlb.so.2,libpcre2-8.so.0,liblz4.so.1,libgcrypt.so.20,libzstd.so.1,libyaml-0.so.2,libxcb.so.1,libwebp.so.6,libjbig.so.0,libicuuc.so.66,libXau.so.6,libXdmcp.so.6,libicudata.so.66,libstdc++.so.6,libbsd.so.0,libz.so.1} $OUT_PATH/usr/lib/$(arch)-linux-gnu/
cp -v  /usr/lib/$(arch)-linux-gnu/gdk-pixbuf-2.0/2.10.0/loaders/libpixbufloader-*.so $OUT_PATH/usr/lib/$(arch)-linux-gnu/gdk-pixbuf-2.0/2.10.0/loaders/
cp -v  /usr/lib/$(arch)-linux-gnu/gdk-pixbuf-2.0/2.10.0/loaders.cache $OUT_PATH/usr/lib/$(arch)-linux-gnu/gdk-pixbuf-2.0/2.10.0/
cp -v  /usr/lib/$(arch)-linux-gnu/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders $OUT_PATH/usr/bin/
cp -v  /usr/bin/{glib-compile-schemas,gtk-update-icon-cache} $OUT_PATH/usr/bin/
