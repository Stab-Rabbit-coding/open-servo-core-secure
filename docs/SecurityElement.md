osc_security.rs integrates an ecc204 Security Element into the osc, consuming the last pin using SWI protocol to connect to the mcu.

The SE requires full asynchronous authentication for firmware updates, calibration and other high risk operations.

For normal operations, the SE authenticates to the controller at boot time and creates an ephemeral shared session key for digitally signing the messages using HMAC-SHA256. Each message is published to the ttl wire with an 8 bit truncation of its HMAC instead of a CRC.

This provides strong integrity and authenticity protection without clobbering the mcu or wire.