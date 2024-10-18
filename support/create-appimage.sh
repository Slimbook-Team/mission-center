#!/bin/bash

# shellcheck disable=SC2164

set -e

if [[ -z "$RECEIPE_PATH" ]]; then
    echo "RECEIPE_PATH is not set or empty"
    exit 1
fi

if [[ -z "$APPDIR_PATH" ]]; then
    echo "APPDIR_PATH is not set or empty"
    exit 1
fi

export HOME=/root
export TERM=xterm
export PATH="/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin:/bin:/sbin:/usr/lib/gcc/$(arch)-linux-gnu/9"
export LD_LIBRARY_PATH="/usr/lib/gcc/$(arch)-linux-gnu/9"

apt-get update

ln -sf /usr/share/zoneinfo/Etc/UTC /etc/localtime
DEBIAN_FRONTEND=noninteractive apt-get install -y tzdata
dpkg-reconfigure --frontend noninteractive tzdata

apt-get install -y python3-pip squashfs-tools zsync librsvg2-2
pip3 install appimage-builder

# https://github.com/AppImageCrafters/appimage-builder/issues/280
# ---------------------------------------------------------------
cat <<EOF > appimage-builder.patch
diff --git a/package.py b/package.py
index 792a724..d8175a4 100644
--- a/python3.8/site-packages/appimagebuilder/modules/deploy/apt/package.py
+++ b/package.py
@@ -76,7 +76,7 @@ class Package:

     def __gt__(self, other):
         if isinstance(other, Package):
-            return version.parse(self.version) > version.parse(other.version)
+            return version.parse(self.version.replace("ubuntu", "")) > version.parse(other.version.replace("ubuntu", ""))

     def __hash__(self):
         return self.__str__().__hash__()

EOF
patch -u /usr/local/lib/python3.8/dist-packages/appimagebuilder/modules/deploy/apt/package.py -i appimage-builder.patch

mv $APPDIR_PATH/usr/bin /helper_binaries
mkdir -p $APPDIR_PATH/usr/bin && mv /helper_binaries/missioncenter* $APPDIR_PATH/usr/bin
cp -rv $APPDIR_PATH/usr/lib/$(arch)-linux-gnu/gdk-pixbuf-2.0 /usr/lib/$(arch)-linux-gnu/

export PATH="/helper_binaries:$PATH"
export LD_LIBRARY_PATH="$APPDIR_PATH/usr/lib/$(arch)-linux-gnu:$LD_LIBRARY_PATH"

sed -i "s|arch: x86_64|arch: $(arch)|g" "$RECEIPE_PATH/io.missioncenter.MissionCenter.yml"
appimage-builder --recipe "$RECEIPE_PATH/io.missioncenter.MissionCenter.yml" --appdir "$APPDIR_PATH" --skip-test

mv Mission\ Center*.AppImage MissionCenter-$(arch).AppImage
