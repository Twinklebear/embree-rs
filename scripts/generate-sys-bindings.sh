#!/bin/bash

bindgen $1 -o $2 \
	--no-doc-comments \
	--distrust-clang-mangling \
	--allowlist-function "rtc.*" \
	--allowlist-type "RTC.*" \
	--allowlist-var "RTC.*" \
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

# Fix up the size_t and ssize_t typedefs
sed -i "s/pub type size_t = ::std::os::raw::c_ulong/pub type size_t = usize/" $2
sed -i "s/pub type __ssize_t = ::std::os::raw::c_long/pub type __ssize_t = isize/" $2

