import b from 'benny'

function add(a: number) {
  return a + 100
}

// TODO: FIXME
async function run() {
  await b.suite(
    'Add 100',

    b.add('Native a + 100', () => {
      // TODO
    }),

    b.add('JavaScript a + 100', () => {
      add(10)
    }),

    b.cycle(),
    b.complete(),
  )
}

run().catch((e) => {
  console.error(e)
})
