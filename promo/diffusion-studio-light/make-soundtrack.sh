#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "$0")" && pwd)"
source_dir="$project_dir/source-audio"
output_file="$source_dir/tfinder-light-bed.wav"
temp_file="$source_dir/tfinder-light-bed.tmp.wav"

mkdir -p "$source_dir" "$project_dir/assets"

pad='0.020*sin(2*PI*110*t)+0.013*sin(2*PI*164.81*t)+0.009*sin(2*PI*220*t)+0.006*sin(2*PI*329.63*t)'
pluck='0.060*sin(2*PI*440*t)*exp(-9*mod(t\,2))*between(mod(t\,2)\,0\,0.46)+0.054*sin(2*PI*523.25*t)*exp(-9*(mod(t\,2)-0.5))*between(mod(t\,2)\,0.5\,0.96)+0.058*sin(2*PI*659.25*t)*exp(-9*(mod(t\,2)-1))*between(mod(t\,2)\,1\,1.46)+0.050*sin(2*PI*523.25*t)*exp(-9*(mod(t\,2)-1.5))*between(mod(t\,2)\,1.5\,1.96)'
rhythm='0.090*sin(2*PI*(72-18*mod(t\,0.5))*t)*exp(-18*mod(t\,0.5))+0.015*sin(2*PI*1900*t)*exp(-75*mod(t+0.25\,0.5))'
accents='0.095*sin(2*PI*130*t)*exp(-8*(t-2))*between(t\,2\,2.8)+0.105*sin(2*PI*130*t)*exp(-8*(t-5))*between(t\,5\,5.8)+0.105*sin(2*PI*130*t)*exp(-8*(t-9))*between(t\,9\,9.8)+0.110*sin(2*PI*120*t)*exp(-8*(t-13))*between(t\,13\,13.8)+0.120*sin(2*PI*105*t)*exp(-7*(t-17))*between(t\,17\,18)'

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "aevalsrc=${pad}|${pad}:s=48000:d=20" \
  -f lavfi -i "aevalsrc=${pluck}|${pluck}:s=48000:d=20" \
  -f lavfi -i "aevalsrc=${rhythm}+${accents}|${rhythm}+${accents}:s=48000:d=20" \
  -f lavfi -i "anoisesrc=color=pink:amplitude=0.018:sample_rate=48000:duration=20" \
  -filter_complex "[0:a]lowpass=f=1100,highpass=f=45,volume=0.62[pad];[1:a]highpass=f=260,lowpass=f=3600,aecho=0.8:0.32:70:0.16,volume=0.72[pluck];[2:a]lowpass=f=5200,highpass=f=42,volume=0.82[rhythm];[3:a]highpass=f=5200,lowpass=f=9800,tremolo=f=4:d=0.76,volume=0.11,pan=stereo|c0=c0|c1=c0[air];[pad][pluck][rhythm][air]amix=inputs=4:normalize=0,volume=1.8,afade=t=in:st=0:d=0.45,afade=t=out:st=18.25:d=1.75,atrim=0:20,alimiter=limit=0.86[out]" \
  -map "[out]" -c:a pcm_s24le "$temp_file"

mv "$temp_file" "$output_file"
ln -sfn "$output_file" "$project_dir/assets/tfinder-light-bed-final.wav"

printf '%s\n' "$output_file"
