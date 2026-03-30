#!/bin/bash
#
# Sync the backend binary & service file to the NAS

scp -O -P 22022 target/release/prono-backend sam@nas:
