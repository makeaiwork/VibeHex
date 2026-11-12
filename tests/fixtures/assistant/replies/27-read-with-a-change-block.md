---
body: I will check your settings first.
reads: settings_get
proposals: labs.smart_spacing=true
---
I will check your settings first.

```hex-read
{"tool":"settings_get","keys":["labs.smart_spacing"]}
```

```hex-change
{"key":"labs.smart_spacing","value":true,"why":"spacing"}
```
