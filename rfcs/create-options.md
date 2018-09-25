# Extending `createOptions` Length
The `createOptions` field in the Edge Agent's twin is part of a module's Docker configuration.
It is an optional field and is used to customize the settings of the Docker container.
Due to the value length limit in the twin, it is restricted to 512 bytes.
Customers have recently been running into this restriction and would like the ability to provide longer `createOptions` on IoT Edge.

## Background
The `createOptions` field allows container's settings to be customized.
For instance, `createOptions` allows a container's mounts, network, port bindings, devices, etc. to be specified.

It is a JSON object with a specific schema.
The schema for this configuration is defined by the Docker Engine, and the IoT Edge runtime cannot modify it.

Due to the lack of arrays in the device twin and the restriction on the depth of objects, this JSON object is stringified and placed into each `module` object of the Edge Agent twin as a field named `createOptions`.

Here is an example of the Edge Agent twin including example `createOptions`.

```json
{
    "systemModules": {
        "edgeAgent": {
            "type": "docker",
            "settings": {
                "image": "mcr.microsoft.com/azureiotedge-agent:1.0.1",
                "createOptions": "{}"
            },
            "env": {
                "RuntimeLogLevel": {
                    "value": "Debug"
                }
            }
        },
        "edgeHub": {
            "type": "docker",
            "settings": {
                "image": "mcr.microsoft.com/azureiotedge-hub:1.0.1",
                "createOptions": "{\"HostConfig\":{\"PortBindings\":{\"8883/tcp\":[{\"HostPort\":\"8883\"}],\"443/tcp\":[{\"HostPort\":\"443\"}],\"5671/tcp\":[{\"HostPort\":\"5671\"}]}}}"
            },
            "status": "running",
            "restartPolicy": "always"
        }
    },
    "modules": {
        "tempsensor": {
            "type": "docker",
            "settings": {
                "image": "mcr.microsoft.com/azureiotedge-simulated-temperature-sensor:1.0.1",
                "createOptions": "{}"
            },
            "version": "1.0",
            "status": "running",
            "restartPolicy": "always"
        }
    }
}
```

Stringifying this JSON object limits its size to 512 bytes.
There are many scenarios where this limit is not enough, particularly when modules need access to hardware on the device.

Since the Edge Agent ultimately owns this schema, changes can be made to increase the effective length of the `createOptions`.

## Proposal

The `createOptions` field in the Edge Agent twin will be chunked into several fields when serializing, with each value less than or equal to the `512b` limit.
These fields will be concatenated when deserializing.

These chunks need to be ordered so that they can be concatenated properly.
However, the device twin does not currently support arrays.
To workaround this limitation, the new fields will append a sequence number to the `createOptions` fields.

To preserve backwards compatibility, the sequence number will start at 1, with the original `createOptions` field remaining unchanged, with an implicit sequence number of 0.
This allows current deployments to function correctly without any changes.
The sequence number consists of two bytes, representing zero-padded integers (`01` - `07`).
This is to allow string compare of these fields.

The sequence numbers must be provided without gaps, and with a limit of 7.
Eight `512b` chunks allows for `4kb` of space, which should be more than enough based on customer feedback.

## Example
Here is an example of a valid Edge Agent twin with extended `createOptions`:

```json
{
    "systemModules": {
        "edgeAgent": {
            "type": "docker",
            "settings": {
                "image": "mcr.microsoft.com/azureiotedge-agent:1.0.1",
                "createOptions": "{}"
            },
            "env": {
                "RuntimeLogLevel": {
                    "value": "Debug"
                }
            }
        },
        "edgeHub": {
            "type": "docker",
            "settings": {
                "image": "mcr.microsoft.com/azureiotedge-hub:1.0.1",
                "createOptions": "{\"HostConfig\":{\"PortBindings\"",
                "createOptions01": ":{\"8883/tcp\":[{\"HostPort\":\"88",
                "createOptions02": "83\"}],\"443/tcp\":[{\"HostPort\":",
                "createOptions03": "\"443\"}],\"5671/tcp\":[{\"HostPort\"",
                "createOptions04": ":\"5671\"}]}}}"
            },
            "status": "running",
            "restartPolicy": "always"
        }
    },
    "modules": {
        "tempsensor": {
            "type": "docker",
            "settings": {
                "image": "mcr.microsoft.com/azureiotedge-simulated-temperature-sensor:1.0.1",
                "createOptions": "{}"
            },
            "version": "1.0",
            "status": "running",
            "restartPolicy": "always"
        }
    }
}
```

The following example is also valid, even though the fields are listed out of order:

```json
{
    "systemModules": {
        "edgeAgent": {
            "type": "docker",
            "settings": {
                "image": "mcr.microsoft.com/azureiotedge-agent:1.0.1",
                "createOptions": "{}"
            },
            "env": {
                "RuntimeLogLevel": {
                    "value": "Debug"
                }
            }
        },
        "edgeHub": {
            "type": "docker",
            "settings": {
                "image": "mcr.microsoft.com/azureiotedge-hub:1.0.1",
                "createOptions": "{\"HostConfig\":{\"PortBindings\"",
                "createOptions03": "\"443\"}],\"5671/tcp\":[{\"HostPort\"",
                "createOptions02": "83\"}],\"443/tcp\":[{\"HostPort\":",
                "createOptions04": ":\"5671\"}]}}}",
                "createOptions01": ":{\"8883/tcp\":[{\"HostPort\":\"88"
            },
            "status": "running",
            "restartPolicy": "always"
        }
    },
    "modules": {
        "tempsensor": {
            "type": "docker",
            "settings": {
                "image": "mcr.microsoft.com/azureiotedge-simulated-temperature-sensor:1.0.1",
                "createOptions": "{}"
            },
            "version": "1.0",
            "status": "running",
            "restartPolicy": "always"
        }
    }
}
```

The following example is *invalid* because the `createOptions` fields are not contiguous:

```json
{
    "systemModules": {
        "edgeAgent": {
            "type": "docker",
            "settings": {
                "image": "mcr.microsoft.com/azureiotedge-agent:1.0.1",
                "createOptions": "{}"
            },
            "env": {
                "RuntimeLogLevel": {
                    "value": "Debug"
                }
            }
        },
        "edgeHub": {
            "type": "docker",
            "settings": {
                "image": "mcr.microsoft.com/azureiotedge-hub:1.0.1",
                "createOptions": "{\"HostConfig\":{\"PortBindings\"",
                "createOptions01": ":{\"8883/tcp\":[{\"HostPort\":\"88",
                "createOptions03": "83\"}],\"443/tcp\":[{\"HostPort\":",
                "createOptions04": "\"443\"}],\"5671/tcp\":[{\"HostPort\"",
                "createOptions05": ":\"5671\"}]}}}"
            },
            "status": "running",
            "restartPolicy": "always"
        }
    },
    "modules": {
        "tempsensor": {
            "type": "docker",
            "settings": {
                "image": "mcr.microsoft.com/azureiotedge-simulated-temperature-sensor:1.0.1",
                "createOptions": "{}"
            },
            "version": "1.0",
            "status": "running",
            "restartPolicy": "always"
        }
    }
}
```