Bindgen command:

```
bindgen <path to embree wrapp.hpp> \
	--whitelist-function "rtc.*" \
	--whitelist-type "RTC.*"
	--whitelist-var "rtc.*" \
	--whitelist-var "RTC.*" \
	--no-doc-comments \
	--distrust-clang-mangling
```

