#!/bin/bash

current_time=$(date +"%Y%m%d%H%M%S")
source_dir="/home/ubuntu/Steam/steamapps/common/PalServer/Pal/Saved"
target_dir="/home/ubuntu/psm/saves"

mkdir -p $target_dir

cd /tmp/ && rm -rf ./Saved && cp -r $source_dir ./

name="Saved.$current_time.tar.gz"

tar -czvf $name Saved

cp $name $target_dir

echo $name
