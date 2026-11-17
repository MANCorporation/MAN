#!/usr/bin/env python3
"""MAN: MAN's own simple menu in the Asahi installer (MIT), used when MAN_AUTO=1
(set by MAN's install-man.sh). Instead of Asahi's partition list and questions:
  1  Install only the MAN boot loader
  2  Install the MAN boot loader and make a partition for MAN   (asks the size)
  3  Make a partition for MAN (boot loader already installed)   (asks the size)
MAN_CHOICE=1|2|3 and MAN_SIZE=<GB> (DUHACH! for Mac) answer these without asking.
Space comes from free space on the disk if there is enough, otherwise macOS's APFS
container is shrunk with Apple's diskutil, never below what Asahi's installer keeps
free for macOS (38 GB free + diskutil's own minimum). Then the normal Asahi steps
(stub macOS, firmware, boot loader, shutdown for the one-time step) run as before.
The OS entries are picked by "man_mode" in installer_data.json (make-usb.sh).
  patch-man-menu.py INSTALLER_TREE"""
import sys
t = sys.argv[1]

def patch(path, pairs):
    s = open(path).read()
    for old, new in pairs:
        if old not in s:
            sys.exit(f"patch-man-menu: pattern not found in {path}:\n{old}")
        s = s.replace(old, new, 1)
    open(path, "w").write(s)
    print("patched", path)

METHODS = r'''
    # ---- MAN: simple menu (MAN_AUTO=1) ----
    def man_menu(self):
        print()
        p_message("MAN for Apple Silicon Macs")
        print()
        p_message("macOS is not changed: only MAN's own partitions are added next to it.")
        p_message("Keep the charger plugged in.")
        print()
        # DUHACH! for Mac asks in its own windows and passes the answer.
        pre = os.environ.get("MAN_CHOICE", "")
        if pre in ("1", "2", "3"):
            p_message(f"Chosen in DUHACH!: {pre}")
            return pre
        p_question("What do you want to do?")
        return self.choice("Choice", {
            "1": "Install only the MAN boot loader (to start the MAN Live USB stick)",
            "2": "Install the MAN boot loader and make a partition for MAN",
            "3": "Make a partition for MAN (the MAN boot loader is already installed)",
            "q": "Quit without changing anything",
        })

    def man_collect(self):
        p_progress("Collecting partition information...")
        self.dutil = diskutil.DiskUtil()
        self.dutil.get_info()
        self.cur_disk = self.sys_disk = self.dutil.find_system_disk()
        self.external_disks = None
        self.parts = self.dutil.get_partitions(self.cur_disk)
        self.osinfo = osenum.OSEnum(self.sysinfo, self.dutil, self.cur_disk)
        self.osinfo.collect(self.parts)
        self.is_edu_128gb = self.dutil.get_disk_size(self.sys_disk) < 150 * 1024 * 1024 * 1024
        self.cur_os = None
        default_os = None
        self.man_loaders = []
        for i, p in enumerate(self.parts):
            p.index = i
            p.desc = p.desc if getattr(p, "desc", None) else ""
            for o in (p.os or []):
                if not o.version:
                    continue
                if self.sysinfo.boot_vgid == o.vgid and self.sysinfo.boot_uuid == o.rec_vgid:
                    if p.type == "APFS":
                        self.cur_os = o
                elif self.sysinfo.boot_uuid == o.vgid:
                    self.cur_os = o
                if self.sysinfo.default_boot == o.vgid:
                    default_os = o
                if o.stub and o.m1n1_ver:
                    self.man_loaders.append(o)
        if self.cur_os is None and self.sysinfo.boot_mode != "macOS":
            self.cur_os = default_os
        self.check_cur_os()
        # macOS's own APFS container (the one that holds the running macOS)
        self.man_macos = None
        for p in self.parts:
            if p.type == "Apple_APFS" and p.container is not None and self.cur_os in (p.os or []):
                self.man_macos = p
        frees = [p for p in self.parts if p.free]
        self.man_free = max(frees, key=lambda p: p.size) if frees else None
        print()

    def man_shrinkable(self):
        """How much macOS's container can give, and its new-size limit."""
        p = self.man_macos
        if p is None:
            return 0, None
        limits = self.dutil.get_resize_limits(p.name)
        total = p.container["CapacityCeiling"]
        free = p.container["CapacityFree"]
        min_size = max(align_up(total - free + self.get_min_free_space(p), PART_ALIGN),
                       limits["MinimumSizePreferred"])
        return max(0, align_down(total - min_size, PART_ALIGN)), total

    def man_make_space(self, need):
        """Free area of at least need bytes; shrinks macOS only when needed."""
        if self.man_free is not None and align_down(self.man_free.size, PART_ALIGN) >= need:
            return self.man_free
        give, total = self.man_shrinkable()
        if give < need:
            p_error(f"Not enough space: macOS can give at most {ssize(give)}.")
            sys.exit(1)
        new = align_down(total - need, PART_ALIGN)
        p_progress(f"Making space: macOS {ssize(total)} -> {ssize(new)} (Apple's diskutil)...")
        p_message("Your Mac may seem to freeze for a moment. This is normal, just wait.")
        target = self.man_macos.name
        self.dutil.resizeContainer(target, new)
        print()
        self.man_collect()
        for i, p in enumerate(self.parts):
            if p.name == target and i + 1 < len(self.parts) and self.parts[i + 1].free:
                if self.parts[i + 1].size >= need:
                    return self.parts[i + 1]
        if self.man_free is not None and self.man_free.size >= need:
            return self.man_free
        raise Exception("The new free space was not found after resizing macOS")

    def man_ask_size(self, fixed, minimum):
        """Asks the size for MAN. fixed = other space needed besides MAN."""
        give, _ = self.man_shrinkable()
        free = align_down(self.man_free.size, PART_ALIGN) if self.man_free else 0
        maximum = align_down(max(give, free) - fixed - psize("16MB"), PART_ALIGN)
        if maximum < minimum:
            p_error(f"Not enough space for MAN: at least {ssize(minimum + fixed)} is needed,")
            p_error(f"but macOS can give only {ssize(max(give, free))}.")
            p_message("Free up space in macOS (delete files, empty the Trash) and try again.")
            sys.exit(1)
        pre = os.environ.get("MAN_SIZE", "")
        if pre:
            try:
                size = align_down(psize(pre.upper() if pre.upper().endswith("B") else pre.upper() + "GB"), PART_ALIGN)
            except Exception:
                size = None
            if size is not None and minimum <= size <= maximum:
                p_message(f"Size for MAN (chosen in DUHACH!): {ssize(size)}")
                print()
                return size
            p_warning(f"The size {pre} is not possible here; please type a size.")
        default = min(maximum, psize("64GB"))
        default = f"{default // 1000**3}GB"
        p_question("How much space should MAN get?")
        p_message(f"  From {ssize(minimum)} to {ssize(maximum)}, for example 60GB.")
        while True:
            size = self.get_size("Size for MAN", default=default, min=minimum, max=maximum)
            if size is None:
                continue
            size = align_down(size, PART_ALIGN)
            if size < minimum:
                p_error(f"Too small: at least {ssize(minimum)}.")
            elif size > maximum:
                p_error(f"Too large: at most {ssize(maximum)}.")
            else:
                print()
                return size

    def man_template(self, mode):
        for t in self.data["os_list"]:
            if str(t.get("man_mode")) == mode:
                return t
        raise Exception(f"installer_data.json has no entry for choice {mode}")

    def man_auto(self, act):
        self.man_collect()
        if act in ("1", "2"):
            template = self.man_template(act)
            self.osins = osinstall.OSInstaller(self.dutil, self.data, template)
            self.osins.load_package()
            base = self.osins.min_size
            total_size = None
            if act == "2":
                exp = [x for x in template["partitions"] if x.get("expand")][0]
                minimum = self.osins.align(psize(exp["size"]))
                base -= minimum
                man_size = self.man_ask_size(STUB_SIZE + base, minimum)
                total_size = base + man_size
            need = STUB_SIZE + (total_size or base)
            ipsw = self.choose_ipsw(template.get("supported_fw", None))
            free_part = self.man_make_space(need + psize("16MB"))
            self.ins = stub.StubInstaller(self.sysinfo, self.dutil, self.osinfo)
            self.ins.load_ipsw(ipsw)
            p_progress(f"Installing the MAN boot loader ({self.osins.name})")
            self.part = self.dutil.addPartition(free_part.name, "apfs", self.osins.name, STUB_SIZE)
            self.do_install(total_size)
            return
        # 3: only the two empty partitions MAN's live installer formats
        # ("Install on This Mac"); macOS and the boot loader are not touched.
        if not self.man_loaders:
            p_error("The MAN boot loader is not installed on this Mac yet.")
            p_message("Run this again and choose 2 (boot loader and partition for MAN).")
            sys.exit(1)
        boot = psize("1GiB")
        man_size = self.man_ask_size(boot, psize("20GB"))
        free_part = self.man_make_space(boot + man_size + psize("16MB"))
        p_progress("Adding partition MAN Boot (1 GiB)...")
        info = self.dutil.addPartition(free_part.name, "%Linux%", "%noformat%", str(boot))
        p_progress(f"Adding partition MAN ({ssize(man_size)})...")
        self.dutil.addPartition(info.name, "%Linux%", "%noformat%", str(man_size))
        print()
        p_success("The space for MAN is ready. macOS was not changed.")
        print()
        p_message("To install MAN:")
        p_message("1. Keep the MAN Live USB stick plugged in.")
        p_message("2. When the Mac has shut down, press and hold the power button, choose MAN.")
        p_message("3. In the MAN Live System: Install MAN > Install on This Mac.")
        print()
        try:
            for i in range(30, 0, -1):
                sys.stdout.write(f"\rShutting down in {i:2d} seconds (Ctrl-C to stay in macOS)... ")
                sys.stdout.flush()
                time.sleep(1)
        except KeyboardInterrupt:
            print()
            p_message("Not shutting down. You can shut down later yourself.")
            return
        print()
        os.system("shutdown -h now")

    def main(self):'''

patch(f"{t}/main.py", [
    ("\n    def main(self):", METHODS),
    ('''    def main(self):
        print()
        p_message(f"Welcome to the {DISTRO} installer!")''',
     '''    def main(self):
        self.man_act = None
        if os.environ.get("MAN_AUTO"):
            self.man_act = self.man_menu()
            if self.man_act == "q":
                return
        else:
          print()
          p_message(f"Welcome to the {DISTRO} installer!")'''),
    ('''        print()
        p_message("This installer will guide you through the process of setting up")
        p_message(f"{DISTRO} on your Mac.")
        print()
        p_message("Please make sure you are familiar with our documentation at:")
        p_plain( f"  {col(BLUE, BRIGHT)}{DISTRO_DOCS}{col()}")
        print()
        p_question("Press enter to continue.")
        self.input()
        print()''',
     '''          print()
          p_message("This installer will guide you through the process of setting up")
          p_message(f"{DISTRO} on your Mac.")
          print()
          p_message("Please make sure you are familiar with our documentation at:")
          p_plain( f"  {col(BLUE, BRIGHT)}{DISTRO_DOCS}{col()}")
          print()
          p_question("Press enter to continue.")
          self.input()
          print()'''),
    ('''        while self.main_loop():
            pass''',
     '''        if self.man_act:
            return self.man_auto(self.man_act)
        while self.main_loop():
            pass'''),
])
