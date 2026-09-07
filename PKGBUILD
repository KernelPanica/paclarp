# Build an installable Arch Linux package with:
#   git archive --format=tar.gz --prefix=paclarp-0.1.0/ HEAD > paclarp-0.1.0.tar.gz
#   makepkg -f
pkgname=paclarp
pkgver=0.1.0
pkgrel=1
pkgdesc='Configurable JSONC-driven pacman output wrapper'
arch=('x86_64' 'aarch64')
url='https://github.com/kernelpanica/paclarp'
license=('GPL3')
depends=('gcc-libs')
makedepends=('cargo')
source=("${pkgname}-${pkgver}.tar.gz")
sha256sums=('SKIP')

build() {
  cd "${srcdir}/${pkgname}-${pkgver}"
  cargo build --release --locked
}

package() {
  cd "${srcdir}/${pkgname}-${pkgver}"
  install -Dm755 "target/release/${pkgname}" "${pkgdir}/usr/bin/${pkgname}"
  install -Dm644 README.md "${pkgdir}/usr/share/doc/${pkgname}/README.md"
  install -Dm644 LICENSE "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}
