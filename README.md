# `grasslands`

![https://github.com/lonelyhentxi/grasslands/actions](https://github.com/lonelyhentxi/grasslands/workflows/CI/badge.svg)

A wrapper with nodejs import resolver and an adapter between [grass](https://github.com/connorskees/grass) and sass-loader? etc.

>**Performance**
> grass is benchmarked against dart-sass and sassc (libsass) here. In general, grass appears to be ~2x faster than dart-sass and ~1.7x faster than sassc.

And In my tests, it is **5~10x** faster than `sass` package.

Thanks to the efforts of the grass authors, this package compiles scss files much faster than the version of `sass` (dart-sass that compiled to js, wildly used version), and without some of the specific problems of `dart-sass-embedded`, such as memory leaks.

**Attention**

Since the current version of [the origin grass](https://github.com/connorskees/grass) has some bugs when dealing with empty argument lists for mixins and functions, and doesn't support custom importers, I'm using [my own forked version of grass](https://github.com/lonelyhentxi/grass) for now, and will switch the upstream of this package to the origin grass if it supports these capabilities in the future.

# Usage

## Install the package

```shell
npm install -D grasslands
```

## Usage for sass-loader

```js
// your other configs
{
  loader: 'sass-loader',
  options: {
    implementation: require.resolve('grasslands/lib/sass-loader-adapter'),
    sassOptions: {
      // to enable thread-loader when use old versions of sass-loader, pass a logger
      logger: {
        debug(message, _loggerOptions) {
          console.debug(message);
        },
        warn(message, _loggerOptions) {
          console.warn(message);
        },
      },
      includePaths: [
        path.resolve('your/include_paths1'),
        path.resolve('your/include_paths2')
      ]
    },
  }
}
// your other configs
```

# Support

### Operating Systems

|                  | node14 | node16 | node18 |
| ---------------- | ------ | ------ | ------ |
| Windows x64      | ✓      | ✓      | ✓      |
| Windows arm64    | ✓      | ✓      | ✓      |
| macOS x64        | ✓      | ✓      | ✓      |
| macOS arm64      | ✓      | ✓      | ✓      |
| Linux x64 gnu    | ✓      | ✓      | ✓      |
| Linux x64 musl   | ✓      | ✓      | ✓      |
| Linux arm64 gnu  | ✓      | ✓      | ✓      |
| Linux arm64 musl | ✓      | ✓      | ✓      |
| FreeBSD x64      | ✓      | ✓      | ✓      |