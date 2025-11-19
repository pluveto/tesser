import logging
import os


def configure_logging():
    level = os.getenv("TESSER_PY_LOG", "INFO").upper()
    logging.basicConfig(
        level=level,
        format="%(asctime)s %(levelname)s [%(name)s] %(message)s",
    )
