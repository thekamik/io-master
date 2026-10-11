/*
 * Table of the devices found by scans, and its display.
 *
 * The table holds one row per Bluetooth address. While a scan
 * runs, the table is updated with fresh data from BlueZ again and
 * again: new devices are appended and the rows of known devices
 * are changed in place. A device never appears twice, and a row
 * never moves or disappears.
 *
 * Device ids
 * ----------
 * The id of a device is the number of its row, counted from 1.
 * Because rows never move, a device keeps its id for as long as
 * it stays in the table, from one scan to the next. The table
 * lives as long as the Bluetooth shell and is not stored on disk.
 * Commands accept the id in place of the Bluetooth address.
 *
 * The table can be shown in two ways:
 *
 *     live     redrawn in place on a separate terminal screen
 *              while the scan runs (bt_scan_screen_*)
 *     plain    printed once as ordinary text (bt_scan_table_print)
 */

#ifndef BLUETOOTH_SCAN_TABLE_H
#define BLUETOOTH_SCAN_TABLE_H

#include <stdbool.h>
#include <stddef.h>

#include "protocol/bluetooth/device.h"

/*
 * One row of the table.
 */
typedef struct {
    /*
     * Last known data of the device. The signal strength
     * is kept from the last time the device was received.
     */
    bt_device_info_t device;

    /*
     * true when the device was found by the current scan, or by
     * the last one when no scan is running. false for a device
     * that is only remembered from earlier scans.
     */
    bool found;

    /*
     * true while BlueZ still receives the device or is connected
     * to it; false when the device is gone. Meaningful only
     * together with found.
     */
    bool in_range;
} bt_scan_entry_t;

/*
 * The table of found devices.
 */
typedef struct {
    bt_scan_entry_t entries[MAX_BT_DEVICES];

    /*
     * Number of used rows. Rows are in the order in which
     * the devices were found; entries[i] has the id i + 1.
     */
    size_t count;

    /*
     * true when a device could not be added because all
     * rows are used.
     */
    bool full;
} bt_scan_table_t;

/*
 * Which rows a display function shows.
 */
typedef enum {
    BT_SCAN_SHOW_FOUND,  /* only the devices found by the last scan */
    BT_SCAN_SHOW_ALL     /* every remembered device */
} bt_scan_filter_t;

/*
 * Empties the table. Has to be called before the table is used.
 * Calling it later forgets all devices; ids start from 1 again.
 */
void bt_scan_table_init(bt_scan_table_t *table);

/*
 * Prepares the table for a new scan: no device counts as found
 * until the scan sees it. The rows and their ids are kept.
 */
void bt_scan_table_begin_scan(bt_scan_table_t *table);

/*
 * Number of devices found by the current or the last scan.
 */
size_t bt_scan_table_found_count(const bt_scan_table_t *table);

/*
 * Looks a device up by its id.
 *
 * Returns NULL when no device has that id.
 */
const bt_scan_entry_t *bt_scan_table_find_by_id(
    const bt_scan_table_t *table,
    size_t id
);

/*
 * Looks a device up by its Bluetooth address. Letter case
 * is ignored. When id is not NULL and the device is found,
 * *id receives its id.
 *
 * Returns NULL when the address is not in the table.
 */
const bt_scan_entry_t *bt_scan_table_find_by_address(
    const bt_scan_table_t *table,
    const char *address,
    size_t *id
);

/*
 * Merges a fresh list of devices from BlueZ into the table.
 *
 * - A device that is already in the table (same address) has
 *   its row updated. Once it is in range, it counts as found.
 * - A new device gets a new row and the next free id, but only
 *   when it is in range: devices that BlueZ merely remembers
 *   are not added.
 * - A row whose device is missing from the list, or no longer
 *   in range, is kept and marked as not in range.
 */
void bt_scan_table_update(
    bt_scan_table_t *table,
    const bt_device_info_t *devices,
    size_t device_count
);

/*
 * Replaces the data of one known device with fresh data, for
 * example after it was connected or paired. A device that is
 * connected or received counts as found and in range.
 *
 * Nothing happens when the device is not in the table.
 */
void bt_scan_table_refresh(
    bt_scan_table_t *table,
    const bt_device_info_t *device
);

/*
 * Prints the table once as ordinary text on stdout.
 * Prints a short message instead when there is nothing to show.
 */
void bt_scan_table_print(
    const bt_scan_table_t *table,
    bt_scan_filter_t filter
);

/*
 * Live display.
 *
 * bt_scan_screen_enter() switches the terminal to a separate
 * screen and bt_scan_screen_leave() switches back, restoring
 * what was shown before. Between the two calls
 * bt_scan_screen_draw() redraws the table in place.
 *
 * These functions write terminal control sequences, so they are
 * meant for a terminal only (see isatty()).
 */
void bt_scan_screen_enter(void);

/*
 * Draws the devices found by the running scan on the live screen.
 *
 * adapter is the name of the scanning card and elapsed_seconds
 * the duration of the scan; both are shown in the title line.
 */
void bt_scan_screen_draw(
    const bt_scan_table_t *table,
    const char *adapter,
    long elapsed_seconds
);

void bt_scan_screen_leave(void);

#endif
