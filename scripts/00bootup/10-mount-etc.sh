mount -t ext4 /dev/disk/by-label/PERSIST /sysroot/persist
mount -t overlay -o upperdir=/sysroot/persist/etc,lowerdir=/sysroot/etc,workdir=/sysroot/persist/etcwork overlay /sysroot/etc

mount --bind /sysroot/persist/var /sysroot/var