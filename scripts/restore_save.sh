#!/bin/bash

# source_dir="/home/ubuntu/Steam/steamapps/common/PalServer/Pal/Saved"
dir="/home/ubuntu/psm/saves"

find $dir -type f | grep "tar.gz" | sort -r | head -n 1 | xargs -I {} cp {} /tmp/
cd /tmp/ && rm -rf ./Saved
tar -zxvf Saved.*.tar.gz
rm -rf /home/ubuntu/Steam/steamapps/common/PalServer/Pal/Saved
cp -r ./Saved /home/ubuntu/Steam/steamapps/common/PalServer/Pal/Saved
rm -rf Saved.*.tar.gz
