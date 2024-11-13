
import Head from 'next/head'

export default function Home() {
  return (
    <div>
      <Head>
        <title>Next.js App</title>
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <main>
        <h1>Welcome to Next.js!</h1>
        <p>Your app is running on port 3000</p>
      </main>
    </div>
  )
}
