# Developer Guide: Custom Field Integration

This guide explains how to integrate custom FIT developer fields and sensors into TrainRS.

## Overview

FIT developer fields allow device manufacturers and third-party applications to embed custom data in FIT files. TrainRS provides a registry system to automatically recognize and parse these fields.

## Quick Start

For a quick introduction to adding custom field support, see [Getting Started](getting-started.md).

## Guide Contents

### Basics
- [Developer Field Basics](developer-field-basics.md) - Understanding FIT developer fields
- [Getting Started](getting-started.md) - Your first custom field integration

### Integration
- [Integration Guide](integration-guide.md) - Step-by-step integration process
- [Sensor Integration](sensor-integration.md) - Custom sensor protocols and patterns
- [Registry System](registry-system.md) - Working with the field registry

### Examples
- [Code Examples](examples/) - Working examples for common scenarios
- [Testing Guide](testing-guide.md) - How to test your integrations

### Reference
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
- [Best Practices](best-practices.md) - Recommendations and patterns

## Prerequisites

- Basic Rust knowledge
- Understanding of FIT file format (see [FIT SDK Documentation](https://developer.garmin.com/fit/overview/))
- Access to device documentation or FIT files with developer fields

## Supported Applications

TrainRS includes built-in support for 12+ popular applications:

- **Stryd** - Running power and dynamics
- **Moxy** - Muscle oxygen monitoring
- **Garmin Running Dynamics** - Advanced running metrics
- **Garmin Vector** - Cycling power with pedal dynamics
- **Wahoo KICKR** - Smart trainer metrics
- **And more...**

See the full list in [src/import/developer_registry.json](../../src/import/developer_registry.json).

## Contributing

When adding support for a new device or application:

1. Add field definitions to `developer_registry.json`
2. Add tests with real device data
3. Update documentation
4. Submit a pull request

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for details.

## Resources

- [FIT SDK Documentation](https://developer.garmin.com/fit/overview/)
- [FIT Protocol](https://developer.garmin.com/fit/protocol/)
- [Developer Field Registration](https://developer.garmin.com/fit/developer-data/)
- [TrainRS API Documentation](https://docs.rs/trainrs)
