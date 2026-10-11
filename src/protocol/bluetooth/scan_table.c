/*
 * Table of the devices found by scans, and its display.
 *
 * The first part of the file maintains the table (adding,
 * updating and looking up devices), the second part prints it.
 *
 * How the live display works
 * --------------------------
 * A terminal understands control sequences: special texts that
 * start with the ESC character (written "\033" in C) and are
 * not shown, but change the state of the terminal. The live
 * table needs only a few of them:
 *
 *     ESC [ ? 1049 h   switch to the alternate screen, a second,
 *                      empty screen without scrollback
 *     ESC [ ? 1049 l   switch back; the previous content of the
 *                      terminal reappears untouched
 *     ESC [ ? 25 l     hide the cursor
 *     ESC [ ? 25 h     show the cursor
 *     ESC [ H          move the cursor to the top left corner
 *     ESC [ K          erase from the cursor to the end of line
 *     ESC [ J          erase from the cursor to the end of screen
 *
 * A redraw moves the cursor to the top left corner and prints
 * the table again over the old one. Every line ends with "erase
 * to end of line", so leftovers of longer old lines disappear,
 * and the redraw ends with "erase to end of screen". The screen
 * is never cleared as a whole, which is why it does not flicker.
 *
 * The same technique is used by programs such as top or watch.
 * No terminal library (ncurses) is needed for it.
 */

#include "protocol/bluetooth/scan_table.h"

#include <stdio.h>
#include <string.h>
#include <strings.h>
#include <sys/ioctl.h>
#include <unistd.h>

/*
 * Terminal control sequences, described at the top of the file.
 */
#define SCREEN_ENTER "\033[?1049h"
#define SCREEN_LEAVE "\033[?1049l"
#define CURSOR_HIDE "\033[?25l"
#define CURSOR_SHOW "\033[?25h"
#define CURSOR_HOME "\033[H"
#define ERASE_LINE "\033[K"
#define ERASE_BELOW "\033[J"

/*
 * Terminal size assumed when the real size is unknown.
 */
#define DEFAULT_ROWS 24
#define DEFAULT_COLUMNS 80

/*
 * Width of everything in a table row that comes before the
 * name column: the fixed columns and the spaces between them
 * (3+2 + 17+2 + 7+2 + 6+2 + 5+2 + 6+2 + 9+2). It has to match
 * ROW_FORMAT and HEADER_FORMAT below.
 */
#define FIXED_COLUMNS_WIDTH 67

/*
 * Narrowest and widest name column. The name column takes the
 * rest of the terminal width, within these limits.
 */
#define NAME_WIDTH_MIN 8
#define NAME_WIDTH_MAX (BT_DEVICE_NAME_MAX - 1)

/*
 * Lines of the live screen that are not table rows: the title,
 * an empty line, the column header and the last line, which is
 * kept free for the "more devices" note.
 */
#define SCREEN_RESERVED_ROWS 4

/*
 * Layout of one table row: id, address, type, address type,
 * signal, paired, state and name. The header uses the same
 * widths, with a text in place of the numeric id.
 */
#define ROW_FORMAT "%3zu  %-17s  %-7s  %-6s  %5s  %-6s  %-9s  %s"
#define HEADER_FORMAT "%3s  %-17s  %-7s  %-6s  %5s  %-6s  %-9s  %s"

void bt_scan_table_init(bt_scan_table_t *table)
{
    memset(table, 0, sizeof(*table));
}

/*
 * Finds the row of the device with the given address.
 * Letter case is ignored: "aa:bb" and "AA:BB" are the same.
 *
 * Returns the index of the row, or table->count when the
 * device is not in the table.
 */
static size_t find_index(
    const bt_scan_table_t *table,
    const char *address
)
{
    for (size_t i = 0; i < table->count; i++) {

        if (strcasecmp(
                table->entries[i].device.address,
                address) == 0) {
            return i;
        }
    }

    return table->count;
}

/*
 * Appends a device in a new row.
 *
 * Returns the new row, or NULL when all rows are used; in that
 * case the table is marked as full.
 */
static bt_scan_entry_t *append_entry(
    bt_scan_table_t *table,
    const bt_device_info_t *device
)
{
    if (table->count >= MAX_BT_DEVICES) {
        table->full = true;
        return NULL;
    }

    bt_scan_entry_t *entry = &table->entries[table->count];

    table->count++;

    entry->device = *device;
    entry->found = false;
    entry->in_range = false;

    return entry;
}

void bt_scan_table_begin_scan(bt_scan_table_t *table)
{
    for (size_t i = 0; i < table->count; i++) {
        table->entries[i].found = false;
        table->entries[i].in_range = false;
    }
}

size_t bt_scan_table_found_count(const bt_scan_table_t *table)
{
    size_t found = 0;

    for (size_t i = 0; i < table->count; i++) {

        if (table->entries[i].found) {
            found++;
        }
    }

    return found;
}

const bt_scan_entry_t *bt_scan_table_find_by_id(
    const bt_scan_table_t *table,
    size_t id
)
{
    /*
     * Ids start at 1; entries[0] has the id 1.
     */
    if (id == 0 || id > table->count) {
        return NULL;
    }

    return &table->entries[id - 1];
}

const bt_scan_entry_t *bt_scan_table_find_by_address(
    const bt_scan_table_t *table,
    const char *address,
    size_t *id
)
{
    size_t index = find_index(table, address);

    if (index >= table->count) {
        return NULL;
    }

    if (id != NULL) {
        *id = index + 1;
    }

    return &table->entries[index];
}

void bt_scan_table_update(
    bt_scan_table_t *table,
    const bt_device_info_t *devices,
    size_t device_count
)
{
    /*
     * Assume that every known device is gone. The loop below
     * marks the ones that are still there.
     */
    for (size_t i = 0; i < table->count; i++) {
        table->entries[i].in_range = false;
    }

    for (size_t i = 0; i < device_count; i++) {
        const bt_device_info_t *device = &devices[i];

        /*
         * BlueZ reports a signal strength only for devices it
         * is receiving right now. A connected device may have
         * none, but it is certainly reachable.
         */
        bool in_range = device->has_rssi || device->connected;

        size_t index = find_index(table, device->address);

        if (index >= table->count) {
            /*
             * A device that is not in the table yet. Devices
             * that BlueZ only remembers from the past are not
             * added.
             */
            if (!in_range) {
                continue;
            }

            bt_scan_entry_t *added = append_entry(table, device);

            if (added != NULL) {
                added->found = true;
                added->in_range = true;
            }

            continue;
        }

        bt_scan_entry_t *entry = &table->entries[index];

        /*
         * A known device that this scan has not seen and that
         * is not in range: it stays a remembered device, with
         * the data of the scan that last saw it.
         */
        if (!entry->found && !in_range) {
            continue;
        }

        /*
         * A known device: replace its data. When the fresh
         * data has no signal strength, keep the last one that
         * was measured.
         */
        bool had_rssi = entry->device.has_rssi;
        int last_rssi = entry->device.rssi;

        entry->device = *device;
        entry->found = true;
        entry->in_range = in_range;

        if (!device->has_rssi && had_rssi) {
            entry->device.has_rssi = true;
            entry->device.rssi = last_rssi;
        }
    }
}

void bt_scan_table_refresh(
    bt_scan_table_t *table,
    const bt_device_info_t *device
)
{
    size_t index = find_index(table, device->address);

    if (index >= table->count) {
        return;
    }

    bt_scan_entry_t *entry = &table->entries[index];

    /*
     * Keep the last measured signal strength when the fresh
     * data has none, as bt_scan_table_update() does.
     */
    bool had_rssi = entry->device.has_rssi;
    int last_rssi = entry->device.rssi;

    entry->device = *device;

    if (!device->has_rssi && had_rssi) {
        entry->device.has_rssi = true;
        entry->device.rssi = last_rssi;
    }

    /*
     * A connected or received device is certainly there, even
     * when the last scan did not see it.
     */
    if (device->connected || device->has_rssi) {
        entry->found = true;
        entry->in_range = true;
    }
}

/*
 * Copies a device name for display.
 *
 * - An empty name is shown as "-".
 * - Control characters are replaced with '?', so a name cannot
 *   contain terminal control sequences of its own.
 * - At most max_columns characters are copied. The name is
 *   UTF-8, so a character can take several bytes; the copy
 *   never stops in the middle of one.
 */
static void format_name(
    const char *name,
    char *buffer,
    size_t buffer_size,
    size_t max_columns
)
{
    size_t length = 0;
    size_t columns = 0;

    if (name[0] == '\0') {
        snprintf(buffer, buffer_size, "-");
        return;
    }

    for (size_t i = 0; name[i] != '\0'; i++) {
        unsigned char byte = (unsigned char)name[i];

        /*
         * In UTF-8 the bytes 10xxxxxx continue a character
         * that started earlier. Every other byte starts
         * a new character.
         */
        bool starts_character = (byte & 0xC0) != 0x80;

        if (starts_character) {

            if (columns >= max_columns) {
                break;
            }

            columns++;
        }

        if (length + 1 >= buffer_size) {
            break;
        }

        /*
         * Control characters: below space, and DEL.
         */
        if (byte < 0x20 || byte == 0x7F) {
            byte = '?';
        }

        buffer[length] = (char)byte;
        length++;
    }

    buffer[length] = '\0';
}

/*
 * Prints the column header. line_end is printed after it:
 * "\n" for plain text, erase-line plus "\n" on the live screen.
 */
static void print_header(const char *line_end)
{
    printf(
        HEADER_FORMAT "%s",
        "ID",
        "ADDRESS",
        "TYPE",
        "ADDR",
        "RSSI",
        "PAIRED",
        "STATE",
        "NAME",
        line_end
    );
}

/*
 * Prints one table row. id is the id of the device, which is
 * its row number. The name is cut to name_width characters.
 */
static void print_row(
    size_t id,
    const bt_scan_entry_t *entry,
    size_t name_width,
    const char *line_end
)
{
    const bt_device_info_t *device = &entry->device;

    char name[BT_DEVICE_NAME_MAX];
    char rssi[8];

    format_name(device->name, name, sizeof(name), name_width);

    if (device->has_rssi) {
        snprintf(rssi, sizeof(rssi), "%d", device->rssi);
    } else {
        snprintf(rssi, sizeof(rssi), "-");
    }

    /*
     * State of the device:
     *
     *     not seen   found by an earlier scan of this session,
     *                but not by the last one
     *     connected  connected to this computer
     *     visible    received by the card right now
     *     lost       seen by the last scan, then gone
     */
    const char *state = "lost";

    if (!entry->found) {
        state = "not seen";
    } else if (device->connected) {
        state = "connected";
    } else if (entry->in_range) {
        state = "visible";
    }

    /*
     * Whether a device from an earlier scan is still paired
     * is not known until a scan asks BlueZ about it.
     */
    const char *paired = "-";

    if (entry->found) {
        paired = device->paired ? "yes" : "no";
    }

    printf(
        ROW_FORMAT "%s",
        id,
        device->address,
        device->classic ? "classic" : "LE",
        device->address_type[0] != '\0' ? device->address_type : "-",
        rssi,
        paired,
        state,
        name,
        line_end
    );
}

/*
 * Tells whether a row passes the filter.
 */
static bool is_shown(
    const bt_scan_entry_t *entry,
    bt_scan_filter_t filter
)
{
    return filter == BT_SCAN_SHOW_ALL || entry->found;
}

void bt_scan_table_print(
    const bt_scan_table_t *table,
    bt_scan_filter_t filter
)
{
    size_t shown = filter == BT_SCAN_SHOW_ALL
        ? table->count
        : bt_scan_table_found_count(table);

    if (shown == 0) {
        printf("No devices.\n");
        return;
    }

    print_header("\n");

    /*
     * Ordinary text is not limited by the screen, so the
     * names are printed in full.
     */
    for (size_t i = 0; i < table->count; i++) {

        if (is_shown(&table->entries[i], filter)) {
            print_row(
                i + 1,
                &table->entries[i],
                NAME_WIDTH_MAX,
                "\n"
            );
        }
    }

    if (table->full) {
        printf(
            "The device list is full (%d devices). New devices "
            "cannot be added until it is emptied with: forget\n",
            MAX_BT_DEVICES
        );
    }
}

void bt_scan_screen_enter(void)
{
    printf(SCREEN_ENTER CURSOR_HIDE);
    fflush(stdout);
}

void bt_scan_screen_leave(void)
{
    printf(CURSOR_SHOW SCREEN_LEAVE);
    fflush(stdout);
}

void bt_scan_screen_draw(
    const bt_scan_table_t *table,
    const char *adapter,
    long elapsed_seconds
)
{
    /*
     * Ask the terminal for its size on every redraw, so the
     * table follows a window that is being resized.
     */
    struct winsize size;
    size_t rows = DEFAULT_ROWS;
    size_t columns = DEFAULT_COLUMNS;

    if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &size) == 0 &&
        size.ws_row > 0 &&
        size.ws_col > 0) {

        rows = size.ws_row;
        columns = size.ws_col;
    }

    /*
     * The name column gets what is left of the line. One column
     * is kept free, because writing into the last column makes
     * some terminals wrap the line.
     */
    size_t name_width = NAME_WIDTH_MIN;

    if (columns > FIXED_COLUMNS_WIDTH + NAME_WIDTH_MIN + 1) {
        name_width = columns - FIXED_COLUMNS_WIDTH - 1;
    }

    if (name_width > NAME_WIDTH_MAX) {
        name_width = NAME_WIDTH_MAX;
    }

    /*
     * Number of devices that fit on the screen. Printing more
     * lines than the screen has would make it scroll.
     */
    size_t visible_rows = 1;

    if (rows > SCREEN_RESERVED_ROWS + 1) {
        visible_rows = rows - SCREEN_RESERVED_ROWS;
    }

    /*
     * Only the devices found by this scan are drawn. Devices
     * remembered from earlier scans would push them off screen.
     */
    size_t found = bt_scan_table_found_count(table);

    /*
     * Redraw from the top left corner, over the old content.
     */
    printf(CURSOR_HOME);

    printf(
        "Scanning on %s | %zu devices | %lds | "
        "press Ctrl+C to stop" ERASE_LINE "\n"
        ERASE_LINE "\n",
        adapter,
        found,
        elapsed_seconds
    );

    print_header(ERASE_LINE "\n");

    /*
     * Draw the found devices until the screen is full.
     */
    size_t drawn = 0;

    for (size_t i = 0; i < table->count && drawn < visible_rows; i++) {

        if (!table->entries[i].found) {
            continue;
        }

        print_row(
            i + 1,
            &table->entries[i],
            name_width,
            ERASE_LINE "\n"
        );

        drawn++;
    }

    /*
     * Last line: tell the user about devices that are not shown.
     * It has no "\n", so the screen never scrolls.
     */
    if (found > drawn) {
        printf(
            "... %zu more (enlarge the window; the full list "
            "is printed after the scan)",
            found - drawn
        );
    } else if (found == 0) {
        printf("Waiting for devices...");
    }

    /*
     * Remove what is left of a previous, longer table.
     */
    printf(ERASE_LINE ERASE_BELOW);

    fflush(stdout);
}
