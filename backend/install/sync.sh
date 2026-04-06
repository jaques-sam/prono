#!/bin/bash
#
# Sync the backend binary & service file to the NAS

scp -O target/release/prono-backend sam@nas:
