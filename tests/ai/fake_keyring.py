#!/usr/bin/env python3
"""
Stand-in for the keyring of the system, used by the AI shell tests.

io-master keeps API keys in the Secret Service of the desktop
(GNOME Keyring, KWallet). A test must never read or change the
real keyring of the user, so this script pretends to be the
Secret Service instead: it takes the D-Bus name
"org.freedesktop.secrets" on the private bus of the test and keeps
the stored secrets in memory.

Only the part of the Secret Service API that libsecret needs for
looking a password up and for storing one is implemented:

    /org/freedesktop/secrets
        Service.OpenSession, SearchItems, Unlock, GetSecrets,
        ReadAlias
    /org/freedesktop/secrets/aliases/default
    /org/freedesktop/secrets/collection/default
        Collection.CreateItem
    /org/freedesktop/secrets/collection/default/<n>
        Item.GetSecret

Secrets are exchanged with the "plain" session algorithm, that is
without the extra encryption a real keyring offers. That is fine
here: the bus is private and the secrets are test values.

Every stored secret is written to a log file, so the tests can
check what io-master stored.

Usage:
    fake_keyring.py LOG_FILE [KEY_NAME=SECRET ...]

Each KEY_NAME=SECRET argument stores a secret in advance, as if the
user had saved an API key earlier. The script prints "ready" once
it can be used and then runs until it is terminated.

Requires the python3-dbus and python3-gi packages.
"""

import sys

import dbus
import dbus.mainloop.glib
import dbus.service
from gi.repository import GLib

SERVICE_PATH = "/org/freedesktop/secrets"
COLLECTION_PATH = SERVICE_PATH + "/collection/default"
ALIAS_PATH = SERVICE_PATH + "/aliases/default"
SESSION_PATH = SERVICE_PATH + "/session/plain"

SERVICE_INTERFACE = "org.freedesktop.Secret.Service"
COLLECTION_INTERFACE = "org.freedesktop.Secret.Collection"
ITEM_INTERFACE = "org.freedesktop.Secret.Item"
PROPERTIES_INTERFACE = "org.freedesktop.DBus.Properties"

# D-Bus type of a secret: session path, parameters of the
# encryption (unused with "plain"), the value, its content type.
SECRET_SIGNATURE = "(oayays)"

# Attributes that io-master puts on its keyring entries.
APPLICATION = "io-master"


class NotSupported(dbus.DBusException):
    """Error that makes libsecret fall back to the plain algorithm."""

    _dbus_error_name = "org.freedesktop.DBus.Error.NotSupported"


class Keyring:
    """The stored items and the log, shared by all objects."""

    def __init__(self, bus, log_path):
        self.bus = bus
        self.log_file = open(log_path, "w", buffering=1)
        self.items = []

    def log(self, text):
        self.log_file.write(text + "\n")

    def find(self, attributes):
        """Returns the items that have all the given attributes."""
        return [
            item for item in self.items
            if all(
                item.attributes.get(name) == value
                for name, value in attributes.items()
            )
        ]

    def store(self, label, attributes, value):
        """Adds an item, replacing one with the same attributes."""
        for item in self.find(attributes):
            if item.attributes == attributes:
                item.value = value
                return item

        item = Item(self, len(self.items) + 1, label, attributes, value)
        self.items.append(item)

        return item


def plain_secret(value):
    """Packs a secret value for the plain session."""
    return dbus.Struct(
        (
            dbus.ObjectPath(SESSION_PATH),
            dbus.ByteArray(b""),
            dbus.ByteArray(value),
            "text/plain",
        ),
        signature="oayays",
    )


class Item(dbus.service.Object):
    """One stored secret (org.freedesktop.Secret.Item)."""

    def __init__(self, keyring, number, label, attributes, value):
        self.keyring = keyring
        self.label = label
        self.attributes = attributes
        self.value = value
        self.path = "%s/%d" % (COLLECTION_PATH, number)

        super().__init__(keyring.bus, self.path)

    @dbus.service.method(
        ITEM_INTERFACE, in_signature="o", out_signature=SECRET_SIGNATURE
    )
    def GetSecret(self, session):
        return plain_secret(self.value)

    @dbus.service.method(
        PROPERTIES_INTERFACE, in_signature="s", out_signature="a{sv}"
    )
    def GetAll(self, interface):
        return {
            "Attributes": dbus.Dictionary(self.attributes, signature="ss"),
            "Label": self.label,
            "Locked": dbus.Boolean(False),
            "Created": dbus.UInt64(0),
            "Modified": dbus.UInt64(0),
        }


class Collection(dbus.service.Object):
    """
    The default keyring (org.freedesktop.Secret.Collection).

    libsecret addresses it through the alias path, so the same
    class is published at both the real path and the alias.
    """

    def __init__(self, keyring, path):
        self.keyring = keyring
        super().__init__(keyring.bus, path)

    @dbus.service.method(
        COLLECTION_INTERFACE,
        in_signature="a{sv}" + SECRET_SIGNATURE + "b",
        out_signature="oo",
    )
    def CreateItem(self, properties, secret, replace):
        label = str(properties.get(ITEM_INTERFACE + ".Label", ""))
        attributes = {
            str(name): str(value)
            for name, value in properties.get(
                ITEM_INTERFACE + ".Attributes", {}
            ).items()
        }

        # The third element of the secret is the value.
        value = bytes(secret[2])

        item = self.keyring.store(label, attributes, value)

        self.keyring.log(
            "store key_name=%s secret=%s"
            % (attributes.get("key_name", ""), value.decode())
        )

        # "/" as the second result means "no prompt needed".
        return dbus.ObjectPath(item.path), dbus.ObjectPath("/")

    @dbus.service.method(
        PROPERTIES_INTERFACE, in_signature="s", out_signature="a{sv}"
    )
    def GetAll(self, interface):
        return {
            "Items": dbus.Array(
                [dbus.ObjectPath(item.path) for item in self.keyring.items],
                signature="o",
            ),
            "Label": "Default",
            "Locked": dbus.Boolean(False),
            "Created": dbus.UInt64(0),
            "Modified": dbus.UInt64(0),
        }


class Service(dbus.service.Object):
    """The entry point (org.freedesktop.Secret.Service)."""

    def __init__(self, keyring):
        self.keyring = keyring
        super().__init__(keyring.bus, SERVICE_PATH)

    @dbus.service.method(
        SERVICE_INTERFACE, in_signature="sv", out_signature="vo"
    )
    def OpenSession(self, algorithm, _input):
        # libsecret first asks for an encrypted session and falls
        # back to "plain" when that is not supported.
        if algorithm != "plain":
            raise NotSupported("only the plain algorithm is supported")

        return dbus.String("", variant_level=1), dbus.ObjectPath(SESSION_PATH)

    @dbus.service.method(
        SERVICE_INTERFACE, in_signature="a{ss}", out_signature="aoao"
    )
    def SearchItems(self, attributes):
        wanted = {str(name): str(value) for name, value in attributes.items()}
        found = [dbus.ObjectPath(item.path) for item in self.keyring.find(wanted)]

        self.keyring.log(
            "search key_name=%s found=%d"
            % (wanted.get("key_name", ""), len(found))
        )

        # Unlocked items, then locked items: nothing is ever locked.
        return dbus.Array(found, signature="o"), dbus.Array([], signature="o")

    @dbus.service.method(
        SERVICE_INTERFACE, in_signature="ao", out_signature="aoo"
    )
    def Unlock(self, objects):
        return dbus.Array(objects, signature="o"), dbus.ObjectPath("/")

    @dbus.service.method(
        SERVICE_INTERFACE,
        in_signature="aoo",
        out_signature="a{o" + SECRET_SIGNATURE + "}",
    )
    def GetSecrets(self, items, session):
        by_path = {item.path: item for item in self.keyring.items}

        return dbus.Dictionary(
            {
                dbus.ObjectPath(path): plain_secret(by_path[str(path)].value)
                for path in items
                if str(path) in by_path
            },
            signature="o" + SECRET_SIGNATURE,
        )

    @dbus.service.method(
        SERVICE_INTERFACE, in_signature="s", out_signature="o"
    )
    def ReadAlias(self, name):
        if name == "default":
            return dbus.ObjectPath(COLLECTION_PATH)

        return dbus.ObjectPath("/")

    @dbus.service.method(
        PROPERTIES_INTERFACE, in_signature="s", out_signature="a{sv}"
    )
    def GetAll(self, interface):
        return {
            "Collections": dbus.Array(
                [dbus.ObjectPath(COLLECTION_PATH)], signature="o"
            )
        }


def main():
    if len(sys.argv) < 2:
        sys.exit("usage: fake_keyring.py LOG_FILE [KEY_NAME=SECRET ...]")

    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)

    # The private bus of the test is its "session" bus, which is
    # where the Secret Service lives on a real system too.
    bus = dbus.SessionBus()

    # Keep a reference: the name is released when it is collected.
    bus_name = dbus.service.BusName("org.freedesktop.secrets", bus)

    keyring = Keyring(bus, sys.argv[1])

    Service(keyring)
    Collection(keyring, COLLECTION_PATH)
    Collection(keyring, ALIAS_PATH)

    # Secrets stored in advance, with the attributes that
    # io-master uses for its entries.
    for argument in sys.argv[2:]:
        key_name, _, secret = argument.partition("=")

        keyring.store(
            "IO Master API key: " + key_name,
            {
                "application": APPLICATION,
                "key_name": key_name,
                "xdg:schema": "io.master.ApiKey",
            },
            secret.encode(),
        )

    # Tell the test that the fake can be used now.
    print("ready", flush=True)

    GLib.MainLoop().run()

    del bus_name


if __name__ == "__main__":
    main()
