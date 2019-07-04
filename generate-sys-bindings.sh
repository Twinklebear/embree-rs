#!/bin/bash

# This should be run on nightly rust, as it requires
# rustfmt-nightly to do the formatting

bindgen $1 -o $2 \
	--no-doc-comments \
	--distrust-clang-mangling \
	--whitelist-function "rtc.*" \
	--whitelist-type "RTC.*" \
	--whitelist-var "RTC.*" \
	--rustified-enum "RTCDeviceProperty" \
	--rustified-enum "RTCError" \
	--rustified-enum "RTCBufferType" \
	--rustified-enum "RTCGeometryType" \
	--rustified-enum "RTCSubdivisionMode" \
	--rustified-enum "RTCFormat" \
	--rustified-enum "RTCBuildQuality" \
	--bitfield-enum "RTC.*Flags" \
	--rust-target nightly

# Run some sed to polish up the enums
sed -i "s/RTC_FORMAT_//g" $2
sed -i "s/RTC_BUILD_QUALITY_//g" $2
sed -i "s/RTC_DEVICE_PROPERTY_//g" $2
sed -i "s/RTC_ERROR_//g" $2
sed -i "s/RTC_BUFFER_TYPE_//g" $2
sed -i "s/RTC_GEOMETRY_TYPE_//g" $2
sed -i "s/RTC_SUBDIVISION_MODE_//g" $2

# And the bitflags
sed -i "s/RTC_INTERSECT_CONTEXT_FLAG_//g" $2
sed -i "s/RTC_CURVE_FLAG_//g" $2
sed -i "s/RTC_SCENE_FLAG_//g" $2
sed -i "s/RTC_BUILD_FLAG_//g" $2


