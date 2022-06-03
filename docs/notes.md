# Notes

Useful command to convert .jpg to .png

```bash
find . -name "*.jpg" -exec mogrify -format png {} \;
```

Create and compare bench baselines, see [critcmp](https://github.com/BurntSushi/critcmp)

```bash
cargo bench -- -save-baseline before 
cargo bench -- -save-baseline change
critcmp before change
```
