# Security and IPC

[Documentation index](README.md)

## Privilege boundary

The background service owns the writable charge controller and machine-wide policy. CLI and GUI run as ordinary applications. They request a fixed set of operations: status, capabilities, compatibility reports, configuration reads, validated limits, temporary top-up and bounded discharge.

The service exposes local IPC only. Its commands accept typed charging settings rather than shell commands, filesystem paths, SMC keys or WMI method names. Both clients and service validate request values, and each backend checks its hardware-specific constraints.

## Local access

On Unix, sockets are created in an administrator-owned directory that is not group/world writable. Socket mode is `0660`. Manual Linux installation grants access to root and the `batterybridge` group. Native Linux packages instead use the existing local administrator group (`sudo` for Debian, `wheel` for RPM), without changing membership; macOS grants access to root and the local `admin` group. Group membership permits changes to the machine's charging policy, shared by all users.

An OS file lock protects Unix endpoint ownership. Startup can reclaim a stale socket after confirming that no listener is active. Active listeners and non-socket paths are preserved.

Windows uses a named pipe with a protected DACL. Network logons are denied. SYSTEM and Administrators have full access, and interactive local users have read/write access. Any authorized interactive user can change the machine-wide charging policy.

The Windows installer rejects reparse points and trees owned by other users before copying code. It replaces complete file/directory DACLs so that executable and state directories are writable only by Administrators/SYSTEM. The implementation uses safe `interprocess` security-descriptor wrappers. WMI queries are local.

## Protocol

The current request protocol is version 2. Messages are Serde JSON with a four-byte big-endian length prefix and a maximum payload of 64 KiB. Frame reads and writes each have a three-second deadline. On Windows, an empty nonblocking transfer waits for progress within that deadline. Connections are configured once before exchanging frames.

At most eight accepted requests wait for the single policy thread. Hardware operations are serialized. The listener blocks while idle, and the policy thread waits for requests, power events or its 30-second reconciliation interval. Client traffic does not indefinitely postpone policy reconciliation. On macOS, queued mutations are rejected while the service handles sleep.

Upgrade CLI, GUI and service together when the protocol changes. Incompatible versions return an explicit error. Read-only fallback is limited to status and capabilities when the service is unavailable or access is denied; configuration reads and mutations require the service.

In an offline status response, `config` is null and all policy state, including temporary modes, is unavailable. Clients must not interpret the temporary-mode placeholder fields as current state.

## Policy persistence

Configuration stores desired settings, not temporary hardware state. The service applies a validated change and persists it before acknowledging success. If persistence fails, it attempts to restore the previous policy.

Writes use an exclusively created, synced sibling file and same-filesystem replacement. An existing transaction file is preserved for recovery. See [configuration](configuration.md) and [service recovery](service.md#troubleshooting) for operational details.

## Hardware safety

Private SMC interfaces can reject writes even with root privileges. Read-only discovery does not establish write permission. Platform authorization and firmware protections remain in effect.

Force discharge has a minimum target of 20%. There is no public indefinite adapter-disable command. The service releases isolation before macOS sleep and on orderly shutdown. Firmware changes and abrupt termination can still affect charging state; supervised validation is described in the [hardware guide](hardware.md#device-validation).

## Diagnostic privacy

Compatibility reports use an allowlist of device metadata and interface results. They exclude serials, usernames, network identifiers, installation IDs and raw WMI instance paths. Logs avoid raw provider payloads and identifiers. Review reports and logs before sharing, especially when adding information collected outside BatteryBridge.

## Linux GUI dependency backport

The Linux GUI uses GTK3 through Tauri. Its glib 0.18.5 dependency is affected by
[GHSA-wrw7-89jp-8q8g](https://github.com/advisories/GHSA-wrw7-89jp-8q8g), an invalid
output pointer in string-variant iteration that can cause undefined behavior
under optimization. BatteryBridge uses the upstream fix through a local source
override; see [provenance and maintenance details](../vendor/README.md). The
version number remains unchanged, so Dependabot may still report the advisory.
The macOS and Windows dependency graphs do not use this Linux GTK dependency.
