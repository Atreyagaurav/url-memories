# Maintainer: Gaurav Atreya <allmanpride@gmail.com>
pkgname=url-memories
pkgver=0.1.1
pkgrel=1
pkgdesc="Remember the URLS for episode/chapter based links"
arch=('x86_64')
url="https://github.com/Atreyagaurav/${pkgname}"
license=('GPL3')
# need to figure out what is make depend and not
depends=('gcc-libs' 'gtk4')
makedepends=('rust' 'cargo')

build() {
	cargo build --release
}

package() {
    cd "$srcdir"
    mkdir -p "$pkgdir/usr/bin"
    cp "../target/release/${pkgname}" "$pkgdir/usr/bin/${pkgname}"
    mkdir -p "$pkgdir/usr/share/applications/"
    cp "../${pkgname}.desktop" "$pkgdir/usr/share/applications/${pkgname}.desktop"
    mkdir -p "$pkgdir/usr/share/${pkgname}/"
    cp "../resources/window.ui" "$pkgdir/usr/share/${pkgname}/"
}
