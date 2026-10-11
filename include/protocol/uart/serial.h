#ifndef SERIAL_H
#define SERIAL_H

#include <stddef.h>
#include <sys/types.h>

#define SERIAL_DEVICE_MAX 256

typedef enum {
    PARITY_NONE,
    PARITY_EVEN,
    PARITY_ODD
} parity_t;

typedef struct {
    char device[SERIAL_DEVICE_MAX];
    int baudrate;
    int data_bits;
    parity_t parity;
    int stop_bits;
} serial_config_t;

typedef struct {
    int fd;
    serial_config_t config;
} serial_port_t;

/*
 * Otwiera port i konfiguruje go zgodnie z config.
 *
 * Zwraca:
 *   0  - sukces
 *  -1  - błąd
 */
int serial_start_connection(
    serial_port_t *port,
    const serial_config_t *config
);

/*
 * Odczytuje maksymalnie size bajtów.
 *
 * Zwraca:
 *   >0 - liczba odczytanych bajtów
 *    0 - brak danych / koniec zgodnie z konfiguracją
 *   -1 - błąd
 */
ssize_t serial_read(
    serial_port_t *port,
    void *buffer,
    size_t size
);

/*
 * Zapisuje maksymalnie size bajtów.
 *
 * Zwraca:
 *   >=0 - liczba zapisanych bajtów
 *   -1  - błąd
 */
ssize_t serial_write(
    serial_port_t *port,
    const void *buffer,
    size_t size
);

/*
 * Zamyka port.
 *
 * Zwraca:
 *    0 - sukces
 *   -1 - błąd
 */
int serial_end_connection(serial_port_t *port);

#endif