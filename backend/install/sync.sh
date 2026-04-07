#!/bin/bash
#
# Sync the backend binary & service file to the NAS

scp -O target/"${1:-release}"/prono-backend sam@nas:
