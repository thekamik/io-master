#ifndef UART_H
#define UART_H

#include "protocol/protocol.h"

/*
 * UART protocol: reads data from a serial port.
 *
 * The settings of a UART project (device, baud rate, data bits,
 * parity, stop bits) are stored in its settings file.
 */
extern const protocol_t uart_protocol;

#endif
