MADARA_DEVNET_NAME = ""
MADARA_DEVNET_IMAGE = ""
MADARA_GRPC_PORT = ""
MADARA_GRPC_PROTOCOL_NAME = ""

def run(plan):
    devnet = plan.add_service(
        name = MADARA_DEVNET_NAME,
        config = ServiceConfig(
            image = MADARA_DEVNET_IMAGE,
            ports = {
                MADARA_PORT_NAME: PortSpec(
                    number = MADARA_GRPC_PORT,
                    application_protocol = MADARA_GRPC_PROTOCOL_NAME,
                )
            }
        )
    )

    exec_recipe = ExecRecipe(
        command = [
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

    result = plan.exec(
        service_name = MADARA_DEVNET_NAME,
        recipe = exec_recipe
    )

    return