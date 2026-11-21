// Tests duhach-core.js with diskutil output of a simulated M1 Mac with MAN installed.
//   node duhach/mac/test-core.js
const assert = require('assert');
const C = require('./duhach-core.js');
const plist = (body) => `<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0">\n${body}\n</plist>\n`;
const p = (dev, content, size, name) => `<dict><key>Content</key><string>${content}</string><key>DeviceIdentifier</key><string>${dev}</string><key>Size</key><integer>${size}</integer>${name ? `<key>VolumeName</key><string>${name}</string>` : ''}</dict>`;
const G = 1e9;
// slices numbered out of disk order on purpose (diskutil lists by number)
const parts = [
  ['disk0s1', 'Apple_APFS_ISC', 0.5 * G, 0],
  ['disk0s2', 'Apple_APFS', 430 * G, 1],
  ['disk0s5', 'Apple_APFS', 2.5 * G, 2],
  ['disk0s4', 'EFI', 0.5 * G, 3, 'EFI - MAN'],
  ['disk0s6', 'Linux Filesystem', 1 * G, 4],
  ['disk0s7', 'Linux Filesystem', 60 * G, 5],
  ['disk0s3', 'Apple_APFS_Recovery', 5.3 * G, 6],
];
const offsets = {};
let off = 24576;
[...parts].sort((a, b) => a[3] - b[3]).forEach(x => { offsets[x[0]] = off; off += x[2]; });
const fake = {
  'diskutil list -plist disk0': plist(`<dict><key>AllDisksAndPartitions</key><array><dict><key>Content</key><string>GUID_partition_scheme</string><key>DeviceIdentifier</key><string>disk0</string><key>Partitions</key><array>${parts.map(x => p(x[0], x[1], x[2], x[4])).join('')}</array></dict></array></dict>`),
  'diskutil apfs list -plist': plist(`<dict><key>Containers</key><array>
    <dict><key>ContainerReference</key><string>disk3</string><key>PhysicalStores</key><array><dict><key>DeviceIdentifier</key><string>disk0s2</string></dict></array><key>Volumes</key><array><dict><key>DeviceIdentifier</key><string>disk3s1</string><key>Name</key><string>Macintosh HD</string><key>Roles</key><array><string>System</string></array></dict></array></dict>
    <dict><key>ContainerReference</key><string>disk4</string><key>PhysicalStores</key><array><dict><key>DeviceIdentifier</key><string>disk0s5</string></dict></array><key>Volumes</key><array><dict><key>DeviceIdentifier</key><string>disk4s1</string><key>Name</key><string>MAN</string><key>Roles</key><array/></dict><dict><key>DeviceIdentifier</key><string>disk4s2</string><key>Name</key><string>Preboot</string><key>Roles</key><array><string>Preboot</string></array></dict></array></dict>
  </array></dict>`),
  'diskutil info -plist /': plist(`<dict><key>APFSContainerReference</key><string>disk3</string><key>APFSPhysicalStores</key><array><dict><key>APFSPhysicalStore</key><string>disk0s2</string></dict></array><key>MountPoint</key><string>/</string></dict>`),
};
parts.forEach(x => { fake['diskutil info -plist ' + x[0]] = plist(`<dict><key>PartitionMapPartitionOffset</key><integer>${offsets[x[0]]}</integer><key>Size</key><integer>${x[2]}</integer></dict>`); });
const run = (cmd) => { if (!(cmd in fake)) throw new Error('unexpected command: ' + cmd); return fake[cmd]; };

const lay = C.layout(run);
assert.strictEqual(lay.stub.store, 'disk0s5');
assert.strictEqual(lay.stub.volume, 'disk4s1');
assert.strictEqual(lay.esp.dev, 'disk0s4');
assert.deepStrictEqual(lay.linux.map(x => x.dev), ['disk0s6', 'disk0s7']);
assert.strictEqual(lay.macos.store, 'disk0s2');
assert.ok(lay.installed);
assert.strictEqual(C.gb(C.manBytes(lay)), '64 GB');
assert.deepStrictEqual(C.deleteCommands(lay), [
  'diskutil eraseVolume free free disk0s7', 'diskutil eraseVolume free free disk0s6',
  'diskutil eraseVolume free free disk0s4', 'diskutil apfs deleteContainer disk0s5',
  'diskutil apfs resizeContainer disk0s2 0']);

// Only the boot loader (installer choice 1): no Linux partitions.
const fake2 = Object.assign({}, fake);
fake2['diskutil list -plist disk0'] = fake['diskutil list -plist disk0'].replace(/<dict><key>Content<\/key><string>Linux Filesystem<\/string>.*?<\/dict>/g, '');
const lay2 = C.layout(c => { if (!(c in fake2)) throw new Error(c); return fake2[c]; });
assert.ok(lay2.stub && lay2.esp && !lay2.installed);

// No MAN at all.
const fake3 = Object.assign({}, fake);
fake3['diskutil apfs list -plist'] = fake['diskutil apfs list -plist'].replace('<string>MAN</string>', '<string>Other</string>');
const lay3 = C.layout(c => fake3[c]);
assert.ok(!lay3.stub && !lay3.installed);
assert.throws(() => C.deleteCommands(lay3));

// GRUB environment block: exactly 1024 bytes, readable back, matches grub-editenv.
const env = C.envBuild({ man_verbose: '1', man_default: 'man-macos' });
assert.strictEqual(env.length, 1024);
assert.deepStrictEqual(C.envParse(env), { man_default: 'man-macos', man_verbose: '1' });

// 24 hours.
assert.strictEqual(C.deleteState(null, 0).state, 'none');
assert.strictEqual(C.deleteState(1000, 1000 + C.WAIT_MS - 1).state, 'waiting');
assert.strictEqual(C.deleteState(1000, 1000 + C.WAIT_MS).state, 'ready');
console.log('duhach-core: all tests passed');
