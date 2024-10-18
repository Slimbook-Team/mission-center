#!/bin/bash

# shellcheck disable=SC2164

set -e

if [[ -z "$SRC_PATH" ]]; then
    echo "SRC_PATH is not set or empty"
    exit 1
fi

# Adapted from https://raw.githubusercontent.com/snapcore/snapcraft/master/docker/Dockerfile
function install-snap() {
  SNAP_NAME=$1

  echo "Installing $SNAP_NAME..."

  curl -L $(curl -H 'X-Ubuntu-Series: 16' "https://api.snapcraft.io/api/v1/snaps/details/$SNAP_NAME" | jq '.download_url' -r) --output $SNAP_NAME.snap

  mkdir -pv /snap/$SNAP_NAME
  unsquashfs -d /snap/$SNAP_NAME/current $SNAP_NAME.snap || true

  rm $SNAP_NAME.snap
}

export HOME=/root
export TERM=xterm
export PATH="/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin:/bin:/sbin:/usr/lib/gcc/$(arch)-linux-gnu/9"
export LD_LIBRARY_PATH="/usr/lib/gcc/$(arch)-linux-gnu/9"

apt-get update

ln -sf /usr/share/zoneinfo/Etc/UTC /etc/localtime
DEBIAN_FRONTEND=noninteractive apt-get install -y tzdata
dpkg-reconfigure --frontend noninteractive tzdata

apt-get install -y curl ca-certificates jq squashfs-tools
apt-get install -y snapd sudo locales
locale-gen en_US.UTF-8

# Adapted from https://raw.githubusercontent.com/snapcore/snapcraft/master/docker/Dockerfile
install-snap core20
install-snap core22
install-snap gtk-common-themes
install-snap gnome-3-38-2004
install-snap snapcraft

unlink /snap/snapcraft/current/usr/bin/python3
ln -s /snap/snapcraft/current/usr/bin/python3.* /snap/snapcraft/current/usr/bin/python3
echo /snap/snapcraft/current/lib/python3.*/site-packages >> /snap/snapcraft/current/usr/lib/python3/dist-packages/site-packages.pth

mkdir -p /snap/bin
echo "#!/bin/sh" > /snap/bin/snapcraft
snap_version="$(awk '/^version:/{print $2}' /snap/snapcraft/current/meta/snap.yaml | tr -d \')" && echo "export SNAP_VERSION=\"$snap_version\"" >> /snap/bin/snapcraft
echo 'exec "$SNAP/usr/bin/python3" "$SNAP/bin/snapcraft" "$@"' >> /snap/bin/snapcraft
chmod +x /snap/bin/snapcraft

export LANG="en_US.UTF-8"
export LANGUAGE="en_US:en"
export LC_ALL="en_US.UTF-8"
export PATH="/snap/bin:/snap/snapcraft/current/usr/bin:$PATH"
export SNAP="/snap/snapcraft/current"
export SNAP_NAME="snapcraft"
export SNAP_ARCH="$(arch)"

cd $SRC_PATH && snapcraft
mv -v mission-center*.snap mission-center_$(arch).snap
