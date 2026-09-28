#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "$0")" && pwd)"
source_dir="$project_dir/source-audio"
output_file="$source_dir/terfinder-bed.wav"
temp_file="$source_dir/terfinder-bed.tmp.wav"

mkdir -p "$source_dir" "$project_dir/assets"

tone_expr='0.042*sin(2*PI*55*t)+0.021*sin(2*PI*82.41*t)+0.014*sin(2*PI*110*t)*pow(max(0\,1-mod(t\,0.5)*4)\,8)+0.105*sin(2*PI*145*t)*exp(-8*(t-3))*between(t\,3\,3.75)+0.105*sin(2*PI*145*t)*exp(-8*(t-7.4))*between(t\,7.4\,8.15)+0.105*sin(2*PI*145*t)*exp(-8*(t-11.3))*between(t\,11.3\,12.05)+0.105*sin(2*PI*145*t)*exp(-8*(t-15.3))*between(t\,15.3\,16.05)+0.13*sin(2*PI*110*t)*exp(-6*(t-20.6))*between(t\,20.6\,21.6)'

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "aevalsrc=${tone_expr}|${tone_expr}:s=48000:d=25" \
  -f lavfi -i "anoisesrc=color=pink:amplitude=0.018:sample_rate=48000:duration=25" \
  -filter_complex "[0:a]lowpass=f=1600,highpass=f=35,volume=0.82[tone];[1:a]highpass=f=4200,lowpass=f=9800,volume=0.16,pan=stereo|c0=c0|c1=c0[air];[tone][air]amix=inputs=2:normalize=0,afade=t=in:st=0:d=0.6,afade=t=out:st=23.2:d=1.8,alimiter=limit=0.82[out]" \
  -map "[out]" -c:a pcm_s24le "$temp_file"

mv "$temp_file" "$output_file"
ln -sfn "$output_file" "$project_dir/assets/terfinder-bed.wav"

printf '%s\n' "$output_file"
