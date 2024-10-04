MADARA_DEVNET_IMAGE = "tranduy1dol/madara:devnet"
MADARA_DEVNET_SERVICE_NAME = "madara_devnet"

MADARA_GRPC_PORT_NAME = ""
MADARA_GRPC_PORT_NUMBER = ""
MADARA_GRPC_PROTOCOL_NAME = ""

def run(plan):
    devnet = plan.add_service(
        name = MADARA_DEVNET_SERVICE_NAME,
        config = ServiceConfig(
            image = MADARA_DEVNET_IMAGE,
            ports = {
                MADARA_PORT_NAME: PortSpec(
                    number = MADARA_GRPC_PORT_NUMBER,
                    application_protocol = MADARA_GRPC_PROTOCOL_NAME,
                )
            },
        cmd = [
            "/usr/local/bin/madara",
            "--base-path",
            "/var/lib/madara-devnet-db",
            "--network",
            "devnet",
            "--preset",
            "sepolia",
            "--authority",
            "--devnet",
            "--override-devnet-chain-id",
            "--no-l1-sync"
            ]
        )
    )

    madara_service_url = get_service_url(
        MADARA_GRPC_PROTOCOL_NAME,
        devnet,
        MADARA_GRPC_PORT
    )
    return

def get_service_url(protocol, service, api_port):
    return "%s://%s:%d" % (protocol, service.ip_address, api_port)