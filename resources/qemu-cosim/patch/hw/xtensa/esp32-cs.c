/*
 * ESP32 SoC and machine, forked as esp32-cs for Circuit Simulator co-simulation
 *
 * Copyright (c) 2019 Espressif Systems (Shanghai) Co. Ltd.
 * Copyright (c) 2026 Veli Tasali (esp32-cs fork)
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License version 2 or
 * (at your option) any later version.
 *
 * Forked from Espressif's public "esp32" machine (hw/xtensa/esp32.c,
 * github.com/espressif/qemu, branch esp-develop) to add Circuit Simulator's
 * shared-memory co-simulation device over the GPIO/IO_MUX/I2C0-1/HSPI/VSPI/
 * UART0-2/LEDC register windows. Everything else (DPORT, RTC_CNTL, TIMG,
 * EFUSE, RSA/SHA/AES, TWAI, RNG, SPI0/1 flash, RMT, PCNT, I2S, analog/rtcio)
 * stays exactly as Espressif wires it, so ROM boot, flash XIP and the
 * FreeRTOS tick keep working unmodified.
 */

#include "qemu/osdep.h"
#include "qemu/log.h"
#include "qemu/error-report.h"
#include "qemu/units.h"
#include "qapi/error.h"
#include "hw/hw.h"
#include "hw/boards.h"
#include "hw/loader.h"
#include "hw/sysbus.h"
#include "hw/i2c/esp32_i2c.h"
#include "hw/xtensa/xtensa_memory.h"
#include "hw/misc/unimp.h"
#include "hw/irq.h"
#include "hw/i2c/i2c.h"
#include "hw/qdev-properties.h"
#include "hw/xtensa/esp32.h"
#include "hw/misc/ssi_psram.h"
#include "hw/sd/dwc_sdmmc.h"
#include "core-esp32/core-isa.h"
#include "qemu/datadir.h"
#include "qemu/timer.h"
#include "qemu/notify.h"
#include "sysemu/sysemu.h"
#include "sysemu/reset.h"
#include "sysemu/cpus.h"
#include "sysemu/runstate.h"
#include "sysemu/blockdev.h"
#include "sysemu/block-backend.h"
#include "exec/exec-all.h"
#include "net/net.h"
#include "elf.h"

#ifndef _WIN32
#include <sys/mman.h>
#include <fcntl.h>
#else
#include <windows.h>
#endif

/*
 * ---------------------------------------------------------------------------
 * Circuit Simulator co-simulation device.
 *
 * Mirrors qemuArena_t from src/microsim/cores/qemu/qemudevice.h
 * (CircuitSimulator repo) byte-for-byte -- this struct is the wire format for a
 * raw shared memory mapping between this process and Circuit Simulator's, so
 * its layout must never drift from that header without updating both sides
 * together.
 *
 * Protocol, as read off the consumer (qemudevice.cpp/qemumodule.cpp, both in
 * the CircuitSimulator repo -- there is no surviving QEMU-side implementation
 * to match, see the esp32-cs plan/memory for how this was derived):
 *   - regAddr is IOMEM-relative (absolute MMIO address minus DR_REG_DPORT_BASE,
 *     i.e. 0x3ff44000 -> 0x44000), not the raw guest physical address.
 *   - simuAction/regAddr/regData/simuTime is a single-slot mailbox: a new
 *     request may only be posted once the previous one's simuAction has been
 *     cleared back to SIM_NONE by the simulator.
 *   - simuAction carries the doorbell: the simulator spin-waits on simuAction
 *     becoming nonzero (QemuDevice::runEvent()), so it must be the LAST field
 *     written when posting a request -- simuTime/regAddr/regData must all be
 *     written before it, or the simulator can observe the doorbell and read
 *     stale/torn data.
 *   - only SIM_READ produces a response (qemuAction=SIM_READ once regData is
 *     valid); SIM_WRITE is fire-and-forget, matching real posted-write MMIO
 *     semantics -- the simulator applies it whenever its own event queue
 *     reaches that circuit time, with no further QEMU-side involvement.
 *   - irqNumber/irqLevel have no doorbell (deliberately unset by the
 *     simulator, see qemumodule.cpp's commented-out qemuAction=SIM_INTERRUPT)
 *     so QEMU must poll them, not wait for an edge.
 * ---------------------------------------------------------------------------
 */

typedef struct Esp32CsArena {
    uint64_t simuTime; /* ps, QEMU sets before simuAction, simulator clears to 0 */
    uint64_t qemuTime; /* ps, informational */
    uint64_t regData;
    uint64_t regAddr; /* IOMEM-relative, i.e. absolute - DR_REG_DPORT_BASE */
    uint64_t irqNumber; /* ESP32 intmatrix ETS_*_INTR_SOURCE id */
    uint64_t irqLevel;
    uint64_t simuAction; /* Esp32CsSimAction, set by QEMU */
    uint64_t qemuAction; /* Esp32CsSimAction, set by the simulator (SIM_READ only) */
    uint64_t running; /* QEMU sets 1 once attached and ready */
    int64_t loop_timeout_ns; /* unused by the current consumer */
    double ps_per_inst;
} Esp32CsArena;

enum {
    ESP32CS_SIM_NONE = 0,
    ESP32CS_SIM_READ,
    ESP32CS_SIM_WRITE,
    ESP32CS_SIM_FREQ,
    ESP32CS_SIM_INTERRUPT,
};

/* Matches the "-icount shift=4" this machine is always launched with
 * (Esp32::createArgs() in the CircuitSimulator repo) -- informational only, the
 * current consumer doesn't read ps_per_inst back, but keep it honest. */
#define ESP32CS_ICOUNT_SHIFT 4
#define ESP32CS_IRQ_POLL_NS 100000

typedef struct Esp32CsCosim {
    Esp32CsArena *arena;
#ifdef _WIN32
    HANDLE shm_handle;
#else
    int shm_fd;
#endif
    QEMUTimer *irq_poll_timer;
    DeviceState *intmatrix_dev;
    uint64_t last_irq_number;
    uint64_t last_irq_level;
} Esp32CsCosim;

/* One QEMU process only ever drives one Circuit Simulator device, so a
 * single file-scope instance avoids threading a pointer through the
 * machine-init -> soc-realize -> MemoryRegionOps call chain via QOM. */
static Esp32CsCosim esp32cs_cosim;

static int64_t esp32cs_now_ps(void)
{
    int64_t ps = qemu_clock_get_ns(QEMU_CLOCK_VIRTUAL) * (int64_t) 1000;
    /* simuTime==0 means "no event" on the consumer side, so a guest access
     * at virtual time 0 still has to post something nonzero. */
    return ps > 0 ? ps : 1;
}

static bool esp32cs_cosim_attach(const char *shm_key, Error **errp)
{
    if (!shm_key || !shm_key[0]) {
        error_setg(errp, "esp32-cs: the \"cosim-shm\" machine property is required "
                   "(Circuit Simulator's Esp32::createArgs() always passes it -- "
                   "if you're running this machine by hand, pass the same shared "
                   "memory key Circuit Simulator would)");
        return false;
    }
#ifdef _WIN32
    HANDLE hMap = OpenFileMappingA(FILE_MAP_ALL_ACCESS, FALSE, shm_key);
    if (!hMap) {
        error_setg(errp, "esp32-cs: OpenFileMapping(\"%s\") failed: %lu",
                   shm_key, (unsigned long) GetLastError());
        return false;
    }
    void *mem = MapViewOfFile(hMap, FILE_MAP_ALL_ACCESS, 0, 0, sizeof(Esp32CsArena));
    if (!mem) {
        error_setg(errp, "esp32-cs: MapViewOfFile(\"%s\") failed: %lu",
                   shm_key, (unsigned long) GetLastError());
        CloseHandle(hMap);
        return false;
    }
    esp32cs_cosim.arena = (Esp32CsArena *) mem;
    esp32cs_cosim.shm_handle = hMap;
#else
    int fd = shm_open(shm_key, O_RDWR, 0666);
    if (fd < 0) {
        error_setg(errp, "esp32-cs: shm_open(\"%s\") failed: %s", shm_key, strerror(errno));
        return false;
    }
    void *mem = mmap(NULL, sizeof(Esp32CsArena), PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (mem == MAP_FAILED) {
        error_setg(errp, "esp32-cs: mmap of \"%s\" failed: %s", shm_key, strerror(errno));
        close(fd);
        return false;
    }
    esp32cs_cosim.arena = (Esp32CsArena *) mem;
    esp32cs_cosim.shm_fd = fd;
#endif
    memset(esp32cs_cosim.arena, 0, sizeof(Esp32CsArena));
    esp32cs_cosim.arena->ps_per_inst = 1000.0 * (double) (1 << ESP32CS_ICOUNT_SHIFT);
    fprintf(stderr, "esp32-cs: cosim attached, shm=\"%s\" arena=%p size=%zu\n",
            shm_key, (void *) esp32cs_cosim.arena, sizeof(Esp32CsArena));
    return true;
}

#ifndef _WIN32
#include <sched.h>
#endif

static inline void esp32cs_cpu_relax(void)
{
#if defined(__aarch64__) || defined(__arm__)
    __asm__ __volatile__("isb" : : : "memory");
#elif defined(__x86_64__) || defined(_M_X64) || defined(__i386__) || defined(_M_IX86)
    __asm__ __volatile__("pause" : : : "memory");
#else
    __asm__ __volatile__("" : : : "memory");
#endif
}

static inline void esp32cs_spin_wait_step(unsigned int *spin_count)
{
    if (*spin_count < 256) {
        esp32cs_cpu_relax();
        (*spin_count)++;
    } else {
#if defined(_WIN32)
        SwitchToThread();
#else
        sched_yield();
#endif
    }
}

/* Single-slot mailbox: don't clobber a request the simulator hasn't picked
 * up yet. Safe as a plain spin (no lock) because -icount forces QEMU's TCG
 * accelerator into single-threaded round-robin mode, so only one vCPU can
 * ever be inside this code at a time. */
static void esp32cs_wait_mailbox_free(Esp32CsArena *arena)
{
    unsigned int spins = 0;
    while (arena->simuAction != ESP32CS_SIM_NONE) {
        esp32cs_spin_wait_step(&spins);
    }
}

static int esp32cs_trace_budget = 0;

static uint64_t esp32cs_cosim_read(void *opaque, hwaddr addr, unsigned size)
{
    Esp32CsArena *arena = esp32cs_cosim.arena;
    hwaddr rel_base = (hwaddr)(uintptr_t) opaque;
    if (!arena) {
        return 0;
    }

    esp32cs_wait_mailbox_free(arena);

    arena->regAddr = rel_base + addr;
    arena->qemuTime = (uint64_t) esp32cs_now_ps();
    arena->simuTime = arena->qemuTime;
    arena->simuAction = ESP32CS_SIM_READ; /* doorbell -- must be written last, after simuTime */

    if (esp32cs_trace_budget > 0) {
        fprintf(stderr, "esp32-cs: READ  addr=0x%" PRIx64 " -- waiting for ack\n", (uint64_t) arena->regAddr);
    }
    unsigned int spins = 0;
    while (arena->qemuAction != ESP32CS_SIM_READ) {
        esp32cs_spin_wait_step(&spins);
    }
    uint32_t data = (uint32_t) arena->regData;
    arena->qemuAction = ESP32CS_SIM_NONE;
    if (esp32cs_trace_budget > 0) {
        fprintf(stderr, "esp32-cs: READ  addr=0x%" PRIx64 " data=0x%x\n", (uint64_t) arena->regAddr, data);
        esp32cs_trace_budget--;
    }
    return data;
}

static void esp32cs_cosim_write(void *opaque, hwaddr addr, uint64_t data, unsigned size)
{
    Esp32CsArena *arena = esp32cs_cosim.arena;
    hwaddr rel_base = (hwaddr)(uintptr_t) opaque;
    if (!arena) {
        return;
    }

    esp32cs_wait_mailbox_free(arena);

    arena->regAddr = rel_base + addr;
    arena->regData = data;
    arena->qemuTime = (uint64_t) esp32cs_now_ps();
    arena->simuTime = arena->qemuTime;
    arena->simuAction = ESP32CS_SIM_WRITE; /* doorbell -- must be written last, after simuTime; no response expected */
    if (esp32cs_trace_budget > 0) {
        fprintf(stderr, "esp32-cs: WRITE addr=0x%" PRIx64 " data=0x%" PRIx64 "\n", (uint64_t) arena->regAddr, data);
        esp32cs_trace_budget--;
    }
}

static const MemoryRegionOps esp32cs_cosim_ops = {
    .read = esp32cs_cosim_read,
    .write = esp32cs_cosim_write,
    .endianness = DEVICE_NATIVE_ENDIAN,
    .valid = { .min_access_size = 1, .max_access_size = 4 },
    .impl = { .min_access_size = 1, .max_access_size = 4 },
};

/* Mirrors esp32_soc_add_periph_device()'s dual DPORT-view/APB-view mapping so
 * firmware using either address convention reaches the same cosim window. */
static void esp32cs_add_cosim_window(MemoryRegion *dest, const char *name, hwaddr dport_base_addr, size_t size)
{
    hwaddr rel_base = dport_base_addr - DR_REG_DPORT_BASE;

    MemoryRegion *mr = g_new(MemoryRegion, 1);
    memory_region_init_io(mr, NULL, &esp32cs_cosim_ops, (void *) (uintptr_t) rel_base, name, size);
    memory_region_add_subregion_overlap(dest, dport_base_addr, mr, 0);

    MemoryRegion *mr_apb = g_new(MemoryRegion, 1);
    char *name_apb = g_strdup_printf("%s-apb", name);
    memory_region_init_io(mr_apb, NULL, &esp32cs_cosim_ops, (void *) (uintptr_t) rel_base, name_apb, size);
    memory_region_add_subregion_overlap(dest, dport_base_addr - DR_REG_DPORT_APB_BASE + APB_REG_BASE, mr_apb, 0);
    g_free(name_apb);
}

static void esp32cs_irq_poll_timer_cb(void *opaque)
{
    Esp32CsCosim *cs = &esp32cs_cosim;
    if (cs->arena && cs->intmatrix_dev) {
        uint64_t number = cs->arena->irqNumber;
        uint64_t level = cs->arena->irqLevel;
        if ((number != cs->last_irq_number || level != cs->last_irq_level)
            && number < ESP32_INT_MATRIX_INPUTS) {
            qemu_set_irq(qdev_get_gpio_in(cs->intmatrix_dev, (int) number), (int) level);
            cs->last_irq_number = number;
            cs->last_irq_level = level;
        }
    }
    timer_mod(cs->irq_poll_timer, qemu_clock_get_ns(QEMU_CLOCK_VIRTUAL) + ESP32CS_IRQ_POLL_NS);
}

static void esp32cs_start_irq_poll(DeviceState *intmatrix_dev)
{
    esp32cs_cosim.intmatrix_dev = intmatrix_dev;
    esp32cs_cosim.irq_poll_timer = timer_new_ns(QEMU_CLOCK_VIRTUAL, esp32cs_irq_poll_timer_cb, NULL);
    timer_mod(esp32cs_cosim.irq_poll_timer, qemu_clock_get_ns(QEMU_CLOCK_VIRTUAL) + ESP32CS_IRQ_POLL_NS);
}

static void esp32cs_machine_init_done(Notifier *notifier, void *data)
{
    fprintf(stderr, "esp32-cs: machine_init_done, arena=%p\n", (void *) esp32cs_cosim.arena);
    if (esp32cs_cosim.arena) {
        esp32cs_cosim.arena->running = 1;
        fprintf(stderr, "esp32-cs: arena->running=1\n");
    }
}

static Notifier esp32cs_machine_init_done_notifier = {
    .notify = esp32cs_machine_init_done,
};

#define TYPE_ESP32_SOC "xtensa.esp32cs"
#define ESP32_SOC(obj) OBJECT_CHECK(Esp32SocState, (obj), TYPE_ESP32_SOC)

#define TYPE_ESP32_CPU XTENSA_CPU_TYPE_NAME("esp32")



enum {
    ESP32_MEMREGION_IROM,
    ESP32_MEMREGION_DROM,
    ESP32_MEMREGION_DRAM,
    ESP32_MEMREGION_IRAM,
    ESP32_MEMREGION_ICACHE0,
    ESP32_MEMREGION_ICACHE1,
    ESP32_MEMREGION_RTCSLOW,
    ESP32_MEMREGION_RTCFAST_D,
    ESP32_MEMREGION_RTCFAST_I,
    ESP32_MEMREGION_FRAMEBUF,
};

static const struct MemmapEntry {
    hwaddr base;
    hwaddr size;
} esp32_memmap[] = {
    [ESP32_MEMREGION_DROM] = { 0x3ff90000, 0x10000 },
    [ESP32_MEMREGION_IROM] = { 0x40000000, 0x70000 },
    [ESP32_MEMREGION_DRAM] = { 0x3ffae000, 0x52000 },
    [ESP32_MEMREGION_IRAM] = { 0x40080000, 0x40000 },
    [ESP32_MEMREGION_ICACHE0] = { 0x40070000, 0x8000 },
    [ESP32_MEMREGION_ICACHE1] = { 0x40078000, 0x8000 },
    [ESP32_MEMREGION_RTCSLOW] = { 0x50000000, 0x2000 },
    [ESP32_MEMREGION_RTCFAST_I] = { 0x400C0000, 0x2000 },
    [ESP32_MEMREGION_RTCFAST_D] = { 0x3ff80000, 0x2000 },
    /* Virtual Framebuffer, used for the graphical interface */
    [ESP32_MEMREGION_FRAMEBUF] = { 0x20000000, ESP_RGB_MAX_VRAM_SIZE }
};


#define ESP32_SOC_RESET_PROCPU    0x1
#define ESP32_SOC_RESET_APPCPU    0x2
#define ESP32_SOC_RESET_PERIPH    0x4
#define ESP32_SOC_RESET_DIG       (ESP32_SOC_RESET_PROCPU | ESP32_SOC_RESET_APPCPU | ESP32_SOC_RESET_PERIPH)
#define ESP32_SOC_RESET_RTC       0x8
#define ESP32_SOC_RESET_ALL       (ESP32_SOC_RESET_RTC | ESP32_SOC_RESET_DIG)




static void remove_cpu_watchpoints(XtensaCPU* xcs)
{
    for (int i = 0; i < MAX_NDBREAK; ++i) {
        if (xcs->env.cpu_watchpoint[i]) {
            cpu_watchpoint_remove_by_ref(CPU(xcs), xcs->env.cpu_watchpoint[i]);
            xcs->env.cpu_watchpoint[i] = NULL;
        }
    }
}

static void esp32_dig_reset(void *opaque, int n, int level)
{
    Esp32SocState *s = ESP32_SOC(opaque);
    if (level) {
        esp32_dport_clear_ill_trap_state(&s->dport);
        s->requested_reset = ESP32_SOC_RESET_DIG;
        qemu_system_reset_request(SHUTDOWN_CAUSE_GUEST_RESET);
    }
}

static void esp32_cpu_reset(void* opaque, int n, int level)
{
    Esp32SocState *s = ESP32_SOC(opaque);
    if (level) {
        s->requested_reset = (n == 0) ? ESP32_SOC_RESET_PROCPU : ESP32_SOC_RESET_APPCPU;
        /* Use different cause for APP CPU so that its reset doesn't cause QEMU to exit,
         * when -no-reboot option is given.
         */
        ShutdownCause cause = (n == 0) ? SHUTDOWN_CAUSE_GUEST_RESET : SHUTDOWN_CAUSE_SUBSYSTEM_RESET;
        s->rtc_cntl.reset_cause[n] = ESP32_SW_CPU_RESET;
        qemu_system_reset_request(cause);
    }
}

static void esp32_timg_cpu_reset(void* opaque, int n, int level)
{
    Esp32SocState *s = ESP32_SOC(opaque);
    if (level) {
        s->requested_reset = (n == 0) ? ESP32_SOC_RESET_PROCPU : ESP32_SOC_RESET_APPCPU;
        /* Use different cause for APP CPU so that its reset doesn't cause QEMU to exit,
         * when -no-reboot option is given.
         */
        ShutdownCause cause = (n == 0) ? SHUTDOWN_CAUSE_GUEST_RESET : SHUTDOWN_CAUSE_SUBSYSTEM_RESET;
        s->rtc_cntl.reset_cause[n] = ESP32_TGWDT_CPU_RESET;
        qemu_system_reset_request(cause);
    }
}

static void esp32_timg_sys_reset(void* opaque, int n, int level)
{
    Esp32SocState *s = ESP32_SOC(opaque);
    if (level) {
        esp32_dport_clear_ill_trap_state(&s->dport);
        s->requested_reset = ESP32_SOC_RESET_DIG;
        for (int i = 0; i < ESP32_CPU_COUNT; ++i) {
            s->rtc_cntl.reset_cause[i] = ESP32_TG0WDT_SYS_RESET + n;
        }
        qemu_system_reset_request(SHUTDOWN_CAUSE_GUEST_RESET);
    }
}

static void esp32_soc_reset(DeviceState *dev)
{
    Esp32SocState *s = ESP32_SOC(dev);

    uint32_t strap_mode = s->gpio.strap_mode;

    bool flash_boot_mode = ((strap_mode & 0x10) || (strap_mode & 0x1f) == 0x0c);
    qemu_set_irq(qdev_get_gpio_in_named(DEVICE(&s->flash_enc), ESP32_FLASH_ENCRYPTION_DL_MODE_GPIO, 0), !flash_boot_mode);

    if (s->requested_reset == 0) {
        s->requested_reset = ESP32_SOC_RESET_ALL;
    }
    if (s->requested_reset & ESP32_SOC_RESET_RTC) {
        device_cold_reset(DEVICE(&s->rtc_cntl));
    }
    if (s->requested_reset & ESP32_SOC_RESET_PERIPH) {
        device_cold_reset(DEVICE(&s->dport));
        device_cold_reset(DEVICE(&s->intmatrix));
        device_cold_reset(DEVICE(&s->aes));
        device_cold_reset(DEVICE(&s->rsa));
        device_cold_reset(DEVICE(&s->gpio));
        for (int i = 0; i < ESP32_UART_COUNT; ++i) {
            device_cold_reset(DEVICE(&s->uart[i]));
        }
        for (int i = 0; i < ESP32_FRC_COUNT; ++i) {
            device_cold_reset(DEVICE(&s->frc_timer[i]));
        }
        for (int i = 0; i < ESP32_TIMG_COUNT; ++i) {
            device_cold_reset(DEVICE(&s->timg[i]));
        }
        s->timg[0].flash_boot_mode = flash_boot_mode;
        for (int i = 0; i < ESP32_SPI_COUNT; ++i) {
            device_cold_reset(DEVICE(&s->spi[i]));
        }
        for (int i = 0; i < ESP32_I2C_COUNT; i++) {
            device_cold_reset(DEVICE(&s->i2c[i]));
        }
        device_cold_reset(DEVICE(&s->twai));
        device_cold_reset(DEVICE(&s->efuse));
        if (s->eth) {
            device_cold_reset(s->eth);
        }

        device_cold_reset(DEVICE(&s->rgb));
    }
    if (s->requested_reset & ESP32_SOC_RESET_PROCPU) {
        xtensa_select_static_vectors(&s->cpu[0].env, s->rtc_cntl.stat_vector_sel[0]);
        remove_cpu_watchpoints(&s->cpu[0]);
        cpu_reset(CPU(&s->cpu[0]));
    }
    if (s->requested_reset & ESP32_SOC_RESET_APPCPU) {
        xtensa_select_static_vectors(&s->cpu[1].env, s->rtc_cntl.stat_vector_sel[1]);
        remove_cpu_watchpoints(&s->cpu[1]);
        cpu_reset(CPU(&s->cpu[1]));
    }
    s->requested_reset = 0;
}

static void esp32_cpu_stall(void* opaque, int n, int level)
{
    Esp32SocState *s = ESP32_SOC(opaque);

    bool stall;
    if (n == 0) {
        stall = s->rtc_cntl.cpu_stall_state[0];
    } else {
        stall = s->rtc_cntl.cpu_stall_state[1] || s->dport.appcpu_stall_state || (!s->dport.appcpu_clkgate_state);
    }

    if (stall != s->cpu[n].env.runstall) {
        xtensa_runstall(&s->cpu[n].env, stall);
    }
}

static void esp32_clk_update(void* opaque, int n, int level)
{
    Esp32SocState *s = ESP32_SOC(opaque);
    if (!level) {
        return;
    }

    /* APB clock */
    uint32_t apb_clk_freq, cpu_clk_freq;
    if (s->rtc_cntl.soc_clk == ESP32_SOC_CLK_PLL) {
        const uint32_t cpu_clk_mul[] = {1, 2, 3};
        apb_clk_freq = s->rtc_cntl.pll_apb_freq;
        cpu_clk_freq = cpu_clk_mul[s->dport.cpuperiod_sel] * apb_clk_freq;
    } else {
        apb_clk_freq = s->rtc_cntl.xtal_apb_freq;
        cpu_clk_freq = apb_clk_freq;
    }
    qdev_prop_set_int32(DEVICE(&s->frc_timer), "apb_freq", apb_clk_freq);
    qdev_prop_set_int32(DEVICE(&s->timg[0]), "apb_freq", apb_clk_freq);
    qdev_prop_set_int32(DEVICE(&s->timg[1]), "apb_freq", apb_clk_freq);
    clock_update_hz(s->cpu[0].clock, cpu_clk_freq );
    clock_update_hz(s->cpu[1].clock, cpu_clk_freq );
}

static void esp32_soc_add_periph_device(MemoryRegion *dest, void* dev, hwaddr dport_base_addr)
{
    MemoryRegion *mr = sysbus_mmio_get_region(SYS_BUS_DEVICE(dev), 0);
    memory_region_add_subregion_overlap(dest, dport_base_addr, mr, 0);
    MemoryRegion *mr_apb = g_new(MemoryRegion, 1);
    char *name = g_strdup_printf("mr-apb-0x%08x", (uint32_t) dport_base_addr);
    memory_region_init_alias(mr_apb, OBJECT(dev), name, mr, 0, memory_region_size(mr));
    memory_region_add_subregion_overlap(dest, dport_base_addr - DR_REG_DPORT_APB_BASE + APB_REG_BASE, mr_apb, 0);
    g_free(name);
}

static void esp32_soc_add_unimp_device(MemoryRegion *dest, const char* name, hwaddr dport_base_addr, size_t size)
{
    create_unimplemented_device(name, dport_base_addr, size);
    char * name_apb = g_strdup_printf("%s-apb", name);
    create_unimplemented_device(name_apb, dport_base_addr - DR_REG_DPORT_APB_BASE + APB_REG_BASE, size);
    g_free(name_apb);
}

static void esp32_soc_realize(DeviceState *dev, Error **errp)
{
    Esp32SocState *s = ESP32_SOC(dev);
    MachineState *ms = MACHINE(qdev_get_machine());

    const struct MemmapEntry *memmap = esp32_memmap;
    MemoryRegion *sys_mem = get_system_memory();

    MemoryRegion *dram = g_new(MemoryRegion, 1);
    MemoryRegion *iram = g_new(MemoryRegion, 1);
    MemoryRegion *icache0 = g_new(MemoryRegion, 1);
    MemoryRegion *icache1 = g_new(MemoryRegion, 1);
    MemoryRegion *rtcslow = g_new(MemoryRegion, 1);
    MemoryRegion *rtcfast_i = g_new(MemoryRegion, 1);
    MemoryRegion *rtcfast_d = g_new(MemoryRegion, 1);

    for (int i = 0; i < ms->smp.cpus; ++i) {
        assert(i >= 0 && i <= 9);
        MemoryRegion *drom = g_new(MemoryRegion, 1);
        MemoryRegion *irom = g_new(MemoryRegion, 1);

        char name[18];
        snprintf(name, sizeof(name), "esp32.irom.cpu%d", i);
        memory_region_init_rom(irom, NULL, name,
                            memmap[ESP32_MEMREGION_IROM].size, &error_fatal);
        memory_region_add_subregion(&s->cpu_specific_mem[i], memmap[ESP32_MEMREGION_IROM].base, irom);


        snprintf(name, sizeof(name), "esp32.drom.cpu%d", i);
        memory_region_init_alias(drom, NULL, name, irom, 0x60000, memmap[ESP32_MEMREGION_DROM].size);
        memory_region_add_subregion(&s->cpu_specific_mem[i], memmap[ESP32_MEMREGION_DROM].base, drom);
    }

    memory_region_init_ram(dram, NULL, "esp32.dram",
                           memmap[ESP32_MEMREGION_DRAM].size, &error_fatal);
    memory_region_add_subregion(sys_mem, memmap[ESP32_MEMREGION_DRAM].base, dram);

    memory_region_init_ram(iram, NULL, "esp32.iram",
                           memmap[ESP32_MEMREGION_IRAM].size, &error_fatal);
    memory_region_add_subregion(sys_mem, memmap[ESP32_MEMREGION_IRAM].base, iram);

    memory_region_init_ram(icache0, NULL, "esp32.icache0",
                           memmap[ESP32_MEMREGION_ICACHE0].size, &error_fatal);
    memory_region_add_subregion(sys_mem, memmap[ESP32_MEMREGION_ICACHE0].base, icache0);

    memory_region_init_ram(icache1, NULL, "esp32.icache1",
                           memmap[ESP32_MEMREGION_ICACHE1].size, &error_fatal);
    memory_region_add_subregion(sys_mem, memmap[ESP32_MEMREGION_ICACHE1].base, icache1);

    memory_region_init_ram(rtcslow, NULL, "esp32.rtcslow",
                           memmap[ESP32_MEMREGION_RTCSLOW].size, &error_fatal);
    memory_region_add_subregion(sys_mem, memmap[ESP32_MEMREGION_RTCSLOW].base, rtcslow);

    /* RTC Fast memory is only accessible by the PRO CPU */

    memory_region_init_ram(rtcfast_i, NULL, "esp32.rtcfast_i",
                           memmap[ESP32_MEMREGION_RTCSLOW].size, &error_fatal);
    memory_region_add_subregion(&s->cpu_specific_mem[0], memmap[ESP32_MEMREGION_RTCFAST_I].base, rtcfast_i);

    memory_region_init_alias(rtcfast_d, NULL, "esp32.rtcfast_d", rtcfast_i, 0, memmap[ESP32_MEMREGION_RTCFAST_D].size);
    memory_region_add_subregion(&s->cpu_specific_mem[0], memmap[ESP32_MEMREGION_RTCFAST_D].base, rtcfast_d);

    for (int i = 0; i < ms->smp.cpus; ++i) {
        qdev_realize(DEVICE(&s->cpu[i]), NULL, &error_fatal);
    }

    qdev_realize(DEVICE(&s->dport), &s->periph_bus, &error_fatal);
    MemoryRegion* dport_mem = sysbus_mmio_get_region(SYS_BUS_DEVICE(&s->dport), 0);

    memory_region_add_subregion(sys_mem, DR_REG_DPORT_BASE, dport_mem);
    qdev_connect_gpio_out_named(DEVICE(&s->dport), ESP32_DPORT_APPCPU_RESET_GPIO, 0,
                                qdev_get_gpio_in_named(dev, ESP32_RTC_CPU_RESET_GPIO, 1));
    qdev_connect_gpio_out_named(DEVICE(&s->dport), ESP32_DPORT_APPCPU_STALL_GPIO, 0,
                                qdev_get_gpio_in_named(dev, ESP32_RTC_CPU_STALL_GPIO, 1));
    qdev_connect_gpio_out_named(DEVICE(&s->rtc_cntl), ESP32_DPORT_CLK_UPDATE_GPIO, 0,
                                qdev_get_gpio_in_named(dev, ESP32_RTC_CLK_UPDATE_GPIO, 0));

    for (int i = 0; i < ESP32_CPU_COUNT; ++i) {
        char name[16];
        snprintf(name, sizeof(name), "cpu%d", i);
        object_property_set_link(OBJECT(&s->intmatrix), name, OBJECT(qemu_get_cpu(i)), &error_abort);
    }
    qdev_realize(DEVICE(&s->intmatrix), &s->periph_bus, &error_fatal);
    DeviceState* intmatrix_dev = DEVICE(&s->intmatrix);
    memory_region_add_subregion_overlap(dport_mem, ESP32_DPORT_PRO_INTMATRIX_BASE, sysbus_mmio_get_region(SYS_BUS_DEVICE(&s->intmatrix), 0), -1);

    /* Circuit Simulator delivers IRQs to cosimulated peripherals through the
     * shared arena rather than a real device's own sysbus IRQ line, and
     * there's no doorbell for it (see the protocol note up top) -- poll. */
    esp32cs_start_irq_poll(intmatrix_dev);

    bool init_cache_err = false;
    if (s->dport.flash_blk) {
        for (int i = 0; i < ESP32_CPU_COUNT; ++i) {
            Esp32CacheRegionState *drom0 = &s->dport.cache_state[i].drom0;
            memory_region_add_subregion_overlap(&s->cpu_specific_mem[i], drom0->base, &drom0->illegal_access_trap_mem, -2);
            memory_region_add_subregion_overlap(&s->cpu_specific_mem[i], drom0->base, &drom0->mem, -1);
            Esp32CacheRegionState *iram0 = &s->dport.cache_state[i].iram0;
            memory_region_add_subregion_overlap(&s->cpu_specific_mem[i], iram0->base, &iram0->illegal_access_trap_mem, -2);
            memory_region_add_subregion_overlap(&s->cpu_specific_mem[i], iram0->base, &iram0->mem, -1);
        }
        init_cache_err = true;
    }
    if (s->dport.has_psram) {
        for (int i = 0; i < ESP32_CPU_COUNT; ++i) {
            Esp32CacheRegionState *dram1 = &s->dport.cache_state[i].dram1;
            memory_region_add_subregion_overlap(&s->cpu_specific_mem[i], dram1->base, &dram1->illegal_access_trap_mem, -2);
            memory_region_add_subregion_overlap(&s->cpu_specific_mem[i], dram1->base, &dram1->mem, -1);
        }
        init_cache_err = true;
    }
    if (init_cache_err) {
        qdev_connect_gpio_out_named(DEVICE(&s->dport), ESP32_DPORT_CACHE_ILL_IRQ_GPIO, 0,
                                    qdev_get_gpio_in(DEVICE(&s->intmatrix), ETS_CACHE_IA_INTR_SOURCE));
    }

    int n_crosscore_irqs = ESP32_DPORT_CROSSCORE_INT_COUNT;
    object_property_set_int(OBJECT(&s->crosscore_int), "n_irqs", n_crosscore_irqs, &error_abort);
    qdev_realize(DEVICE(&s->crosscore_int), &s->periph_bus, &error_fatal);
    memory_region_add_subregion_overlap(dport_mem, ESP32_DPORT_CROSSCORE_INT_BASE, &s->crosscore_int.iomem, -1);

    for (int index = 0; index < ESP32_DPORT_CROSSCORE_INT_COUNT; ++index) {
        qemu_irq target = qdev_get_gpio_in(DEVICE(&s->intmatrix), ETS_FROM_CPU_INTR0_SOURCE + index);
        assert(target);
        sysbus_connect_irq(SYS_BUS_DEVICE(&s->crosscore_int), index, target);
    }

    qdev_realize(DEVICE(&s->rsa), &s->periph_bus, &error_fatal);
    esp32_soc_add_periph_device(sys_mem, &s->rsa, DR_REG_RSA_BASE);

    qdev_realize(DEVICE(&s->sha), &s->periph_bus, &error_fatal);
    esp32_soc_add_periph_device(sys_mem, &s->sha, DR_REG_SHA_BASE);

    qdev_realize(DEVICE(&s->aes), &s->periph_bus, &error_fatal);
    esp32_soc_add_periph_device(sys_mem, &s->aes, DR_REG_AES_BASE);

    qdev_realize(DEVICE(&s->ledc), &s->periph_bus, &error_fatal);
    /* Circuit Simulator cosimulates LEDC (PWM -> circuit pins). */
    esp32cs_add_cosim_window(sys_mem, "esp32cs.ledc", DR_REG_LEDC_BASE, 0x1000);

    qdev_realize(DEVICE(&s->rtc_cntl), &s->rtc_bus, &error_fatal);
    esp32_soc_add_periph_device(sys_mem, &s->rtc_cntl, DR_REG_RTCCNTL_BASE);

    qdev_connect_gpio_out_named(DEVICE(&s->rtc_cntl), ESP32_RTC_DIG_RESET_GPIO, 0,
                                qdev_get_gpio_in_named(dev, ESP32_RTC_DIG_RESET_GPIO, 0));
    qdev_connect_gpio_out_named(DEVICE(&s->rtc_cntl), ESP32_RTC_CLK_UPDATE_GPIO, 0,
                                qdev_get_gpio_in_named(dev, ESP32_RTC_CLK_UPDATE_GPIO, 0));
    for (int i = 0; i < ms->smp.cpus; ++i) {
        qdev_connect_gpio_out_named(DEVICE(&s->rtc_cntl), ESP32_RTC_CPU_RESET_GPIO, i,
                                    qdev_get_gpio_in_named(dev, ESP32_RTC_CPU_RESET_GPIO, i));
        qdev_connect_gpio_out_named(DEVICE(&s->rtc_cntl), ESP32_RTC_CPU_STALL_GPIO, i,
                                    qdev_get_gpio_in_named(dev, ESP32_RTC_CPU_STALL_GPIO, i));
    }

    qdev_realize(DEVICE(&s->gpio), &s->periph_bus, &error_fatal);
    /* Circuit Simulator cosimulates GPIO -- forward through the cosim device
     * instead of mapping the real esp32_gpio register block. */
    esp32cs_add_cosim_window(sys_mem, "esp32cs.gpio", DR_REG_GPIO_BASE, 0x1000);

    for (int i = 0; i < ESP32_UART_COUNT; ++i) {
        const hwaddr uart_base[] = {DR_REG_UART_BASE, DR_REG_UART1_BASE, DR_REG_UART2_BASE};
        qdev_realize(DEVICE(&s->uart[i]), &s->periph_bus, &error_fatal);
        /* Circuit Simulator cosimulates all 3 UARTs (serial monitor pins). */
        char uart_name[24];
        snprintf(uart_name, sizeof(uart_name), "esp32cs.uart%d", i);
        esp32cs_add_cosim_window(sys_mem, uart_name, uart_base[i], 0x1000);
    }

    for (int i = 0; i < ESP32_FRC_COUNT; ++i) {
        qdev_realize(DEVICE(&s->frc_timer[i]), &s->periph_bus, &error_fatal);

        esp32_soc_add_periph_device(sys_mem, &s->frc_timer[i], DR_REG_FRC_TIMER_BASE + i * ESP32_FRC_TIMER_STRIDE);

        sysbus_connect_irq(SYS_BUS_DEVICE(&s->frc_timer[i]), 0,
                           qdev_get_gpio_in(intmatrix_dev, ETS_TIMER1_INTR_SOURCE + i));
    }

    for (int i = 0; i < ESP32_TIMG_COUNT; ++i) {
        s->timg[i].id = i;

        const hwaddr timg_base[] = {DR_REG_TIMERGROUP0_BASE, DR_REG_TIMERGROUP1_BASE};
        qdev_realize(DEVICE(&s->timg[i]), &s->periph_bus, &error_fatal);

        esp32_soc_add_periph_device(sys_mem, &s->timg[i], timg_base[i]);

        int timg_level_int[] = { ETS_TG0_T0_LEVEL_INTR_SOURCE, ETS_TG1_T0_LEVEL_INTR_SOURCE };
        int timg_edge_int[] = { ETS_TG0_T0_EDGE_INTR_SOURCE, ETS_TG1_T0_EDGE_INTR_SOURCE };
        for (Esp32TimgInterruptType it = TIMG_T0_INT; it < TIMG_INT_MAX; ++it) {
            sysbus_connect_irq(SYS_BUS_DEVICE(&s->timg[i]), it, qdev_get_gpio_in(intmatrix_dev, timg_level_int[i] + it));
            sysbus_connect_irq(SYS_BUS_DEVICE(&s->timg[i]), TIMG_INT_MAX + it, qdev_get_gpio_in(intmatrix_dev, timg_edge_int[i] + it));
        }

        qdev_connect_gpio_out_named(DEVICE(&s->timg[i]), ESP32_TIMG_WDT_CPU_RESET_GPIO, 0,
                                    qdev_get_gpio_in_named(dev, ESP32_TIMG_WDT_CPU_RESET_GPIO, i));
        qdev_connect_gpio_out_named(DEVICE(&s->timg[i]), ESP32_TIMG_WDT_SYS_RESET_GPIO, 0,
                                    qdev_get_gpio_in_named(dev, ESP32_TIMG_WDT_SYS_RESET_GPIO, i));
    }
    s->timg[0].wdt_en_at_reset = true;

    for (int i = 0; i < ESP32_SPI_COUNT; ++i) {
        const hwaddr spi_base[] = {
            DR_REG_SPI0_BASE, DR_REG_SPI1_BASE, DR_REG_SPI2_BASE, DR_REG_SPI3_BASE
        };
        qdev_realize(DEVICE(&s->spi[i]), &s->periph_bus, &error_fatal);

        if (i < 2) {
            /* SPI0/SPI1 are the internal flash controllers the ROM bootstraps
             * through -- stay real QEMU devices, never cosimulated. */
            esp32_soc_add_periph_device(sys_mem, &s->spi[i], spi_base[i]);
            sysbus_connect_irq(SYS_BUS_DEVICE(&s->spi[i]), 0,
                               qdev_get_gpio_in(intmatrix_dev, ETS_SPI0_INTR_SOURCE + i));
        } else {
            /* SPI2 (HSPI) / SPI3 (VSPI): user-facing, Circuit Simulator cosimulates these. */
            char spi_name[24];
            snprintf(spi_name, sizeof(spi_name), "esp32cs.spi%d", i);
            esp32cs_add_cosim_window(sys_mem, spi_name, spi_base[i], 0x1000);
        }
    }

    for (int i = 0; i < ESP32_I2C_COUNT; i++) {
        const hwaddr i2c_base[] = {
            DR_REG_I2C_EXT_BASE, DR_REG_I2C1_EXT_BASE
        };
        qdev_realize(DEVICE(&s->i2c[i]), &s->periph_bus, &error_fatal);

        /* Circuit Simulator cosimulates both I2C controllers. */
        char i2c_name[24];
        snprintf(i2c_name, sizeof(i2c_name), "esp32cs.i2c%d", i);
        esp32cs_add_cosim_window(sys_mem, i2c_name, i2c_base[i], 0x1000);
    }

    /* TWAI model passes intmatrix IRQs to the SJA1000 controller model
     * in realize function. That means that irq linking MUST be
     * performed before realization of TWAI peripheral.
     */
    qdev_realize(DEVICE(&s->twai), &s->periph_bus, &error_fatal);
    esp32_soc_add_periph_device(sys_mem, &s->twai, DR_REG_CAN_BASE);
    sysbus_connect_irq(SYS_BUS_DEVICE(&s->twai), 0,
                       qdev_get_gpio_in(intmatrix_dev, ETS_CAN_INTR_SOURCE));

    qdev_realize(DEVICE(&s->rng), &s->periph_bus, &error_fatal);
    esp32_soc_add_periph_device(sys_mem, &s->rng, ESP32_RNG_BASE);

    qdev_realize(DEVICE(&s->efuse), &s->periph_bus, &error_fatal);
    esp32_soc_add_periph_device(sys_mem, &s->efuse, DR_REG_EFUSE_BASE);
    sysbus_connect_irq(SYS_BUS_DEVICE(&s->efuse), 0,
                       qdev_get_gpio_in(intmatrix_dev, ETS_EFUSE_INTR_SOURCE));

    qdev_realize(DEVICE(&s->flash_enc), &s->periph_bus, &error_abort);
    esp32_soc_add_periph_device(sys_mem, &s->flash_enc, DR_REG_SPI_ENCRYPT_BASE);

    qdev_connect_gpio_out_named(DEVICE(&s->efuse), ESP32_EFUSE_UPDATE_GPIO, 0,
                                qdev_get_gpio_in_named(DEVICE(&s->flash_enc), ESP32_FLASH_ENCRYPTION_EFUSE_UPDATE_GPIO, 0));
    qdev_connect_gpio_out_named(DEVICE(&s->dport), ESP32_DPORT_FLASH_ENC_EN_GPIO, 0,
                                qdev_get_gpio_in_named(DEVICE(&s->flash_enc), ESP32_FLASH_ENCRYPTION_ENC_EN_GPIO, 0));
    qdev_connect_gpio_out_named(DEVICE(&s->dport), ESP32_DPORT_FLASH_DEC_EN_GPIO, 0,
                                qdev_get_gpio_in_named(DEVICE(&s->flash_enc), ESP32_FLASH_ENCRYPTION_DEC_EN_GPIO, 0));

    qdev_realize(DEVICE(&s->sdmmc), &s->periph_bus, &error_abort);
    esp32_soc_add_periph_device(sys_mem, &s->sdmmc, DR_REG_SDMMC_BASE);
    sysbus_connect_irq(SYS_BUS_DEVICE(&s->sdmmc), 0,
                       qdev_get_gpio_in(intmatrix_dev, ETS_SDIO_HOST_INTR_SOURCE));

    /* Provide internal RAM MemoryRegion to the RGB display */
    s->rgb.intram = dram;
    qdev_realize(DEVICE(&s->rgb), &s->periph_bus, &error_abort);
    esp32_soc_add_periph_device(sys_mem, &s->rgb, DR_REG_FRAMEBUF_BASE);
    memory_region_add_subregion_overlap(sys_mem, esp32_memmap[ESP32_MEMREGION_FRAMEBUF].base, &s->rgb.vram, 0);

    esp32_soc_add_unimp_device(sys_mem, "esp32.analog", DR_REG_ANA_BASE, 0x1000);
    esp32_soc_add_unimp_device(sys_mem, "esp32.rtcio", DR_REG_RTCIO_BASE, 0x400);
    esp32_soc_add_unimp_device(sys_mem, "esp32.rtcio", DR_REG_SENS_BASE, 0x400);
    /* Circuit Simulator cosimulates IO_MUX (pin function routing); upstream
     * only ever had an unimplemented-device stub here anyway. */
    esp32cs_add_cosim_window(sys_mem, "esp32cs.iomux", DR_REG_IO_MUX_BASE, 0x1000);
    esp32_soc_add_unimp_device(sys_mem, "esp32.hinf", DR_REG_HINF_BASE, 0x1000);
    esp32_soc_add_unimp_device(sys_mem, "esp32.slc", DR_REG_SLC_BASE, 0x1000);
    esp32_soc_add_unimp_device(sys_mem, "esp32.slchost", DR_REG_SLCHOST_BASE, 0x1000);
    esp32_soc_add_unimp_device(sys_mem, "esp32.apbctrl", DR_REG_APB_CTRL_BASE, 0x1000);
    esp32_soc_add_unimp_device(sys_mem, "esp32.i2s0", DR_REG_I2S_BASE, 0x1000);
    esp32_soc_add_unimp_device(sys_mem, "esp32.i2s1", DR_REG_I2S1_BASE, 0x1000);
    esp32_soc_add_unimp_device(sys_mem, "esp32.rmt", DR_REG_RMT_BASE, 0x1000);
    esp32_soc_add_unimp_device(sys_mem, "esp32.pcnt", DR_REG_PCNT_BASE, 0x1000);

    /* Emulation of a fake register used to mark that the chip is run via QEMU */
    MemoryRegion *apbctrl_mem = g_new(MemoryRegion, 1);
    memory_region_init_ram(apbctrl_mem, NULL, "esp32.apbctrl_date_reg", 8 /* bytes */, &error_fatal);

    /* This register is not used in the real hardware (hardwired to 0), but is still accesible, reading
     * it won't trigger an exception, so we can override it */
    const hwaddr apb_ctrl_emu_reg = DR_REG_APB_CTRL_BASE + 0x78;
    /* Store "QEMU" as a 32-bit value */
    const uint32_t apb_ctrl_emu_val = 0x51454d55;
    /* The memory region must be added before writing to the CPU memory */
    memory_region_add_subregion(sys_mem, apb_ctrl_emu_reg, apbctrl_mem);
    cpu_physical_memory_write(apb_ctrl_emu_reg, &apb_ctrl_emu_val, 4);

    /* Emulation of APB_CTRL_DATE_REG, needed for ECO3 revision detection.
     * This is a small hack to avoid creating a whole new device just to emulate one
     * register.
     */
    const hwaddr apb_ctrl_date_reg = DR_REG_APB_CTRL_BASE + 0x7c;
    uint32_t apb_ctrl_date_reg_val = 0x16042000 | 0x80000000;  /* MSB indicates ECO3 silicon revision */
    cpu_physical_memory_write(apb_ctrl_date_reg, &apb_ctrl_date_reg_val, 4);

    qemu_register_reset((QEMUResetHandler*) esp32_soc_reset, dev);
}

static void esp32_soc_init(Object *obj)
{
    Esp32SocState *s = ESP32_SOC(obj);
    MachineState *ms = MACHINE(qdev_get_machine());
    char name[16];

    MemoryRegion *system_memory = get_system_memory();

    qbus_init(&s->periph_bus, sizeof(s->periph_bus),
                        TYPE_SYSTEM_BUS, DEVICE(s), "esp32-periph-bus");
    qbus_init(&s->rtc_bus, sizeof(s->rtc_bus),
                        TYPE_SYSTEM_BUS, DEVICE(s), "esp32-rtc-bus");

    for (int i = 0; i < ms->smp.cpus; ++i) {
        snprintf(name, sizeof(name), "cpu%d", i);
        object_initialize_child(obj, name, &s->cpu[i], TYPE_ESP32_CPU);

        const uint32_t cpuid[ESP32_CPU_COUNT] = { 0xcdcd, 0xabab };
        s->cpu[i].env.sregs[PRID] = cpuid[i];

        snprintf(name, sizeof(name), "cpu%d-mem", i);
        memory_region_init(&s->cpu_specific_mem[i], NULL, name, UINT32_MAX);

        CPUState* cs = CPU(&s->cpu[i]);
        cs->num_ases = 1;
        cpu_address_space_init(cs, 0, "cpu-memory", &s->cpu_specific_mem[i]);

        MemoryRegion *cpu_view_sysmem = g_new(MemoryRegion, 1);
        snprintf(name, sizeof(name), "cpu%d-sysmem", i);
        memory_region_init_alias(cpu_view_sysmem, NULL, name, system_memory, 0, UINT32_MAX);
        memory_region_add_subregion_overlap(&s->cpu_specific_mem[i], 0, cpu_view_sysmem, 0);
        cs->memory = &s->cpu_specific_mem[i];
    }

    for (int i = 0; i < ESP32_UART_COUNT; ++i) {
        snprintf(name, sizeof(name), "uart%d", i);
        object_initialize_child(obj, name, &s->uart[i], TYPE_ESP32_UART);
    }

    object_property_add_alias(obj, "serial0", OBJECT(&s->uart[0]), "chardev");
    object_property_add_alias(obj, "serial1", OBJECT(&s->uart[1]), "chardev");
    object_property_add_alias(obj, "serial2", OBJECT(&s->uart[2]), "chardev");

    object_initialize_child(obj, "gpio", &s->gpio, TYPE_ESP32_GPIO);

    object_initialize_child(obj, "dport", &s->dport, TYPE_ESP32_DPORT);

    object_initialize_child(obj, "intmatrix", &s->intmatrix, TYPE_ESP32_INTMATRIX);

    object_initialize_child(obj, "crosscore_int", &s->crosscore_int, TYPE_ESP32_CROSSCORE_INT);

    object_initialize_child(obj, "rtc_cntl", &s->rtc_cntl, TYPE_ESP32_RTC_CNTL);

    for (int i = 0; i < ESP32_FRC_COUNT; ++i) {
        snprintf(name, sizeof(name), "frc%d", i);
        object_initialize_child(obj, name, &s->frc_timer[i], TYPE_ESP32_FRC_TIMER);
    }

    for (int i = 0; i < ESP32_TIMG_COUNT; ++i) {
        snprintf(name, sizeof(name), "timg%d", i);
        object_initialize_child(obj, name, &s->timg[i], TYPE_ESP32_TIMG);
    }

    for (int i = 0; i < ESP32_SPI_COUNT; ++i) {
        snprintf(name, sizeof(name), "spi%d", i);
        object_initialize_child(obj, name, &s->spi[i], TYPE_ESP32_SPI);
    }

    for (int i = 0; i < ESP32_I2C_COUNT; ++i) {
        snprintf(name, sizeof(name), "i2c%d", i);
        object_initialize_child(obj, name, &s->i2c[i], TYPE_ESP32_I2C);
    }

    object_initialize_child(obj, "twai", &s->twai, TYPE_ESP32_TWAI);

    object_initialize_child(obj, "rng", &s->rng, TYPE_ESP32_RNG);

    object_initialize_child(obj, "sha", &s->sha, TYPE_ESP32_SHA);

    object_initialize_child(obj, "aes", &s->aes, TYPE_ESP32_AES);

    object_initialize_child(obj, "ledc", &s->ledc, TYPE_ESP32_LEDC);

    object_initialize_child(obj, "rsa", &s->rsa, TYPE_ESP32_RSA);

    object_initialize_child(obj, "efuse", &s->efuse, TYPE_ESP32_EFUSE);

    object_initialize_child(obj, "flash_enc", &s->flash_enc, TYPE_ESP32_FLASH_ENCRYPTION);

    object_initialize_child(obj, "sdmmc", &s->sdmmc, TYPE_DWC_SDMMC);

    object_initialize_child(obj, "rgb", &s->rgb, TYPE_ESP_RGB);

    qdev_init_gpio_in_named(DEVICE(s), esp32_dig_reset, ESP32_RTC_DIG_RESET_GPIO, 1);
    qdev_init_gpio_in_named(DEVICE(s), esp32_cpu_reset, ESP32_RTC_CPU_RESET_GPIO, ESP32_CPU_COUNT);
    qdev_init_gpio_in_named(DEVICE(s), esp32_cpu_stall, ESP32_RTC_CPU_STALL_GPIO, ESP32_CPU_COUNT);
    qdev_init_gpio_in_named(DEVICE(s), esp32_clk_update, ESP32_RTC_CLK_UPDATE_GPIO, 1);
    qdev_init_gpio_in_named(DEVICE(s), esp32_timg_cpu_reset, ESP32_TIMG_WDT_CPU_RESET_GPIO, 2);
    qdev_init_gpio_in_named(DEVICE(s), esp32_timg_sys_reset, ESP32_TIMG_WDT_SYS_RESET_GPIO, 2);
}

static Property esp32_soc_properties[] = {
    DEFINE_PROP_END_OF_LIST(),
};

static void esp32_soc_class_init(ObjectClass *klass, void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);

    dc->realize = esp32_soc_realize;
    device_class_set_props(dc, esp32_soc_properties);
}

static const TypeInfo esp32_soc_info = {
    .name = TYPE_ESP32_SOC,
    .parent = TYPE_DEVICE,
    .instance_size = sizeof(Esp32SocState),
    .instance_init = esp32_soc_init,
    .class_init = esp32_soc_class_init
};

static void esp32_soc_register_types(void)
{
    type_register_static(&esp32_soc_info);
}

type_init(esp32_soc_register_types)


static uint64_t translate_phys_addr(void *opaque, uint64_t addr)
{
    XtensaCPU *cpu = opaque;

    return cpu_get_phys_page_debug(CPU(cpu), addr);
}


struct Esp32MachineState {
    MachineState parent;

    Esp32SocState esp32;
    DeviceState *flash_dev;
    char *cosim_shm; /* "cosim-shm" property: shared memory key from Circuit Simulator */
};
#define TYPE_ESP32_MACHINE MACHINE_TYPE_NAME("esp32-cs")

OBJECT_DECLARE_SIMPLE_TYPE(Esp32MachineState, ESP32_MACHINE)

/* MachineState's QOM parent is plain Object, not DeviceState, so a "-M
 * esp32-cs,cosim-shm=<key>" option is a class property added directly
 * (object_class_property_add_str), not a DEFINE_PROP_STRING/Property array
 * entry -- those only work on DeviceClass-derived types. */
static char *esp32cs_get_cosim_shm(Object *obj, Error **errp)
{
    Esp32MachineState *ms = ESP32_MACHINE(obj);
    return g_strdup(ms->cosim_shm ? ms->cosim_shm : "");
}

static void esp32cs_set_cosim_shm(Object *obj, const char *value, Error **errp)
{
    Esp32MachineState *ms = ESP32_MACHINE(obj);
    g_free(ms->cosim_shm);
    ms->cosim_shm = g_strdup(value);
}


static void esp32_machine_init_spi_flash(Esp32SocState *ss, BlockBackend* blk)
{
    /* "main" flash chip is attached to SPI1, CS0 */
    DeviceState *spi_master = DEVICE(&ss->spi[1]);
    BusState* spi_bus = qdev_get_child_bus(spi_master, "spi");

    /* select the flash chip based on the image size */
    int64_t image_size = blk_getlength(blk);
    const char* flash_chip_model = NULL;
    switch (image_size) {
        case 2 * 1024 * 1024: flash_chip_model = "w25x16"; break;
        case 4 * 1024 * 1024: flash_chip_model = "gd25q32"; break;
        case 8 * 1024 * 1024: flash_chip_model = "gd25q64"; break;
        case 16 * 1024 * 1024: flash_chip_model = "is25lp128"; break;
        default: error_report("Error: only 2, 4, 8, 16 MB flash images are supported"); return;
    }

    DeviceState *flash_dev = qdev_new(flash_chip_model);
    qdev_prop_set_drive(flash_dev, "drive", blk);
    qdev_prop_set_uint8(flash_dev, "cs", 0);
    qdev_realize_and_unref(flash_dev, spi_bus, &error_fatal);
    qdev_connect_gpio_out_named(spi_master, SSI_GPIO_CS, 0,
                                qdev_get_gpio_in_named(flash_dev, SSI_GPIO_CS, 0));
}

static void esp32_machine_init_psram(Esp32SocState *ss, uint32_t size_mbytes)
{
    /* PSRAM attached to SPI1, CS1 */
    DeviceState *spi_master = DEVICE(&ss->spi[1]);
    BusState* spi_bus = qdev_get_child_bus(spi_master, "spi");
    DeviceState *psram = qdev_new(TYPE_SSI_PSRAM);
    qdev_prop_set_uint32(psram, "size_mbytes", size_mbytes);
    qdev_prop_set_uint8(psram, "cs", 1);
    qdev_realize_and_unref(psram, spi_bus, &error_fatal);
    qdev_connect_gpio_out_named(spi_master, SSI_GPIO_CS, 1,
                                qdev_get_gpio_in_named(psram, SSI_GPIO_CS, 0));
}

static void esp32_machine_init_i2c(Esp32SocState *s)
{
    /* It should be possible to create an I2C device from the command line,
     * however for this to work the I2C bus must be reachable from sysbus-default.
     * At the moment the peripherals are added to an unrelated bus, to avoid being
     * reset on CPU reset.
     * If we find a way to decouple peripheral reset from sysbus reset,
     * we can move them to the sysbus and thus enable creation of i2c devices.
     */
    DeviceState *i2c_master = DEVICE(&s->i2c[0]);
    I2CBus* i2c_bus = I2C_BUS(qdev_get_child_bus(i2c_master, "i2c"));
    I2CSlave* tmp105 = i2c_slave_create_simple(i2c_bus, "tmp105", 0x48);
    object_property_set_int(OBJECT(tmp105), "temperature", 25 * 1000, &error_fatal);
}

static void esp32_machine_init_openeth(Esp32SocState *ss)
{
    SysBusDevice *sbd;
    MemoryRegion* sys_mem = get_system_memory();
    hwaddr reg_base = DR_REG_EMAC_BASE;
    hwaddr desc_base = reg_base + 0x400;
    qemu_irq irq = qdev_get_gpio_in(DEVICE(&ss->intmatrix), ETS_ETH_MAC_INTR_SOURCE);

    DeviceState* open_eth_dev = qemu_create_nic_device("open_eth", true, NULL);
    if (!open_eth_dev) {
        return;
    }

    ss->eth = open_eth_dev;
    sbd = SYS_BUS_DEVICE(open_eth_dev);
    sysbus_realize_and_unref(sbd, &error_fatal);
    sysbus_connect_irq(sbd, 0, irq);
    memory_region_add_subregion(sys_mem, reg_base, sysbus_mmio_get_region(sbd, 0));
    memory_region_add_subregion(sys_mem, desc_base, sysbus_mmio_get_region(sbd, 1));
}

static void esp32_machine_init_sd(Esp32SocState *ss)
{
    DriveInfo *dinfo = drive_get(IF_SD, 0, 0);
    if (dinfo) {
        DeviceState *card;

        card = qdev_new(TYPE_SD_CARD);
        qdev_prop_set_drive_err(card, "drive", blk_by_legacy_dinfo(dinfo),
                                &error_fatal);
        /* See the comment on not using sysbus-default in esp32_machine_init_i2c */
        DeviceState *sdmmc = DEVICE(&ss->sdmmc);
        SDBus* sd_bus = SD_BUS(qdev_get_child_bus(sdmmc, "sd-bus"));
        qdev_realize_and_unref(card, BUS(sd_bus), &error_fatal);
    }
}

static void esp32_machine_init(MachineState *machine)
{
    /* &error_fatal means esp32cs_cosim_attach() never actually returns on
     * failure -- it prints and exit(1)s from inside error_setg(). */
    esp32cs_cosim_attach(ESP32_MACHINE(machine)->cosim_shm, &error_fatal);
    qemu_add_machine_init_done_notifier(&esp32cs_machine_init_done_notifier);

    BlockBackend* blk = NULL;
    DriveInfo *dinfo = drive_get(IF_MTD, 0, 0);
    if (dinfo) {
        qemu_log("Adding SPI flash device\n");
        blk = blk_by_legacy_dinfo(dinfo);
    } else {
        qemu_log("Not initializing SPI Flash\n");
    }

    Esp32MachineState *ms = ESP32_MACHINE(machine);
    object_initialize_child(OBJECT(ms), "soc", &ms->esp32, TYPE_ESP32_SOC);
    Esp32SocState *ss = ESP32_SOC(&ms->esp32);

    if (blk) {
        ss->dport.flash_blk = blk;
    }
    qdev_prop_set_chr(DEVICE(ss), "serial0", serial_hd(0));
    qdev_prop_set_chr(DEVICE(ss), "serial1", serial_hd(1));
    qdev_prop_set_chr(DEVICE(ss), "serial2", serial_hd(2));
    if (machine->ram_size > 0) {
        qdev_prop_set_bit(DEVICE(&ss->dport), "has_psram", true);
    }

    qdev_realize(DEVICE(ss), NULL, &error_fatal);

    if (blk) {
        esp32_machine_init_spi_flash(ss, blk);
    }

    if (machine->ram_size > 0) {
        esp32_machine_init_psram(ss, (uint32_t) (machine->ram_size / MiB));
    }

    esp32_machine_init_i2c(ss);

    esp32_machine_init_openeth(ss);

    esp32_machine_init_sd(ss);

    /* Need MMU initialized prior to ELF loading,
     * so that ELF gets loaded into virtual addresses
     */
    cpu_reset(CPU(&ss->cpu[0]));

    const char *load_elf_filename = NULL;
    if (machine->firmware) {
        load_elf_filename = machine->firmware;
    }
    if (machine->kernel_filename) {
        qemu_log("Warning: both -bios and -kernel arguments specified. Only loading the the -kernel file.\n");
        load_elf_filename = machine->kernel_filename;
    }

    if (load_elf_filename) {
        uint64_t elf_entry;
        uint64_t elf_lowaddr;
        int size = load_elf(load_elf_filename, NULL,
                               translate_phys_addr, &ss->cpu[0],
                               &elf_entry, &elf_lowaddr,
                               NULL, NULL, 0, EM_XTENSA, 0, 0);
        if (size < 0) {
            error_report("Error: could not load ELF file '%s'", load_elf_filename);
            exit(1);
        }

        if (elf_entry != XCHAL_RESET_VECTOR_PADDR) {
            // Since ROM is empty when loading elf file AND
            // PC value is 0x40000400 after reset
            // need to jump to elf entry point to run a programm
            uint8_t p[4];
            memcpy(p, &elf_entry, 4);
            uint8_t boot[] = {
                0x06, 0x01, 0x00,       /* j    1 */
                0x00,                   /* .literal_position */
                p[0], p[1], p[2], p[3], /* .literal elf_entry */
                                        /* 1: */
                0x01, 0xff, 0xff,       /* l32r a0, elf_entry */
                0xa0, 0x00, 0x00,       /* jx   a0 */
            };
            // Write boot function to reset-vector address (0x40000400) of the CPU 0
            rom_add_blob_fixed_as("boot", boot, sizeof(boot), XCHAL_RESET_VECTOR_PADDR, CPU(&ss->cpu[0])->as);
            ss->cpu[0].env.pc = XCHAL_RESET_VECTOR_PADDR;
        }
    } else {
        char *rom_binary = qemu_find_file(QEMU_FILE_TYPE_BIOS, "esp32-v3-rom.bin");
        if (rom_binary == NULL) {
            error_report("Error: -bios argument not set, and ROM code binary not found (1)");
            exit(1);
        }

        int size = load_image_targphys_as(rom_binary, esp32_memmap[ESP32_MEMREGION_IROM].base, esp32_memmap[ESP32_MEMREGION_IROM].size, CPU(&ss->cpu[0])->as);
        if (size < 0) {
            error_report("Error: could not load ROM binary '%s'", rom_binary);
            exit(1);
        }
        g_free(rom_binary);

        rom_binary = qemu_find_file(QEMU_FILE_TYPE_BIOS, "esp32-v3-rom-app.bin");
        if (rom_binary == NULL) {
            error_report("Error: -bios argument not set, and ROM code binary not found (2)");
            exit(1);
        }

        size = load_image_targphys_as(rom_binary, esp32_memmap[ESP32_MEMREGION_IROM].base, esp32_memmap[ESP32_MEMREGION_IROM].size, CPU(&ss->cpu[1])->as);
        if (size < 0) {
            error_report("Error: could not load ROM binary '%s'", rom_binary);
            exit(1);
        }
        g_free(rom_binary);
    }
}

static ram_addr_t esp32_fixup_ram_size(ram_addr_t requested_size)
{
    ram_addr_t size;
    if (requested_size == 0) {
        size = 0;
    } else if (requested_size <= 2 * MiB) {
        size = 2 * MiB;
    } else if (requested_size <= 4 * MiB ) {
        size = 4 * MiB;
    } else {
        qemu_log("RAM size larger than 4 MB not supported\n");
        size = 4 * MiB;
    }
    return size;
}

/* Initialize machine type */
static void esp32_machine_class_init(ObjectClass *oc, void *data)
{
    MachineClass *mc = MACHINE_CLASS(oc);
    mc->desc = "Circuit Simulator ESP32 co-simulation machine";
    mc->init = esp32_machine_init;
    mc->max_cpus = 2;
    mc->default_cpus = 2;
    mc->default_ram_size = 0;
    mc->fixup_ram_size = esp32_fixup_ram_size;

    object_class_property_add_str(oc, "cosim-shm", esp32cs_get_cosim_shm, esp32cs_set_cosim_shm);
    object_class_property_set_description(oc, "cosim-shm",
        "Shared memory key for Circuit Simulator's co-simulation arena "
        "(qemuArena_t) -- required, passed by Esp32::createArgs()");
}

static const TypeInfo esp32_info = {
    .name = TYPE_ESP32_MACHINE,
    .parent = TYPE_MACHINE,
    .instance_size = sizeof(Esp32MachineState),
    .class_init = esp32_machine_class_init,
};

static void esp32_machine_type_init(void)
{
    type_register_static(&esp32_info);
}

type_init(esp32_machine_type_init);
