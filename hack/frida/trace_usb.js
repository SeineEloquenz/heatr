/**
 * Frida script: trace USB bulk transfers in the heat it Android app.
 *
 * Usage:
 *   frida -U -n "heat it" -l trace_usb.js
 *   frida -U -f de.ka.kamedi.heat_it -l trace_usb.js
 *
 * Hooks all UsbDeviceConnection.bulkTransfer() overloads and logs each
 * transfer with direction, endpoint address, and hex-encoded payload.
 */

"use strict";

function toHex(bytes) {
    if (!bytes || bytes.length === 0) return "(empty)";
    var out = [];
    for (var i = 0; i < bytes.length; i++) {
        out.push(("0" + (bytes[i] & 0xff).toString(16)).slice(-2));
    }
    return out.join(" ");
}

function endpointInfo(endpoint) {
    try {
        var UsbEndpoint = Java.use("android.hardware.usb.UsbEndpoint");
        // Direction: 0 = OUT (host→device), 128 = IN (device→host)
        var dir = endpoint.getDirection();
        var addr = endpoint.getAddress();
        var dirStr = (dir === 0) ? "OUT" : "IN";
        return "ep=0x" + ("0" + addr.toString(16)).slice(-2) + " dir=" + dirStr;
    } catch (e) {
        return "ep=?";
    }
}

Java.perform(function () {
    var UsbDeviceConnection = Java.use("android.hardware.usb.UsbDeviceConnection");

    // Overload 1: bulkTransfer(UsbEndpoint, byte[], int, int)
    // Used for transfers without offset.
    UsbDeviceConnection.bulkTransfer.overload(
        "android.hardware.usb.UsbEndpoint",
        "[B",
        "int",
        "int"
    ).implementation = function (endpoint, buffer, length, timeout) {
        var epInfo = endpointInfo(endpoint);
        var dir = endpoint.getDirection();
        var isOut = (dir === 0);

        var preview = null;
        if (isOut && buffer && length > 0) {
            var bytes = [];
            for (var i = 0; i < length && i < buffer.length; i++) {
                bytes.push(buffer[i] & 0xff);
            }
            preview = toHex(bytes);
        }

        var ret = this.bulkTransfer(endpoint, buffer, length, timeout);

        if (isOut) {
            console.log("[USB] WRITE " + epInfo + " len=" + length + "  data: " + (preview || "(none)"));
        } else {
            // For IN transfers the buffer is populated after the call.
            var recvBytes = [];
            var recvLen = ret > 0 ? ret : 0;
            for (var j = 0; j < recvLen && j < buffer.length; j++) {
                recvBytes.push(buffer[j] & 0xff);
            }
            console.log("[USB] READ  " + epInfo + " len=" + recvLen + "  data: " + toHex(recvBytes));
        }

        return ret;
    };

    // Overload 2: bulkTransfer(UsbEndpoint, byte[], int, int, int)
    // Used for transfers with an offset into the buffer.
    UsbDeviceConnection.bulkTransfer.overload(
        "android.hardware.usb.UsbEndpoint",
        "[B",
        "int",
        "int",
        "int"
    ).implementation = function (endpoint, buffer, offset, length, timeout) {
        var epInfo = endpointInfo(endpoint);
        var dir = endpoint.getDirection();
        var isOut = (dir === 0);

        var preview = null;
        if (isOut && buffer && length > 0) {
            var bytes = [];
            for (var i = offset; i < offset + length && i < buffer.length; i++) {
                bytes.push(buffer[i] & 0xff);
            }
            preview = toHex(bytes);
        }

        var ret = this.bulkTransfer(endpoint, buffer, offset, length, timeout);

        if (isOut) {
            console.log("[USB] WRITE " + epInfo + " offset=" + offset + " len=" + length + "  data: " + (preview || "(none)"));
        } else {
            var recvBytes = [];
            var recvLen = ret > 0 ? ret : 0;
            for (var j = offset; j < offset + recvLen && j < buffer.length; j++) {
                recvBytes.push(buffer[j] & 0xff);
            }
            console.log("[USB] READ  " + epInfo + " offset=" + offset + " len=" + recvLen + "  data: " + toHex(recvBytes));
        }

        return ret;
    };

    console.log("[*] USB bulk transfer hooks installed. Interact with the heat it device now.");
});
