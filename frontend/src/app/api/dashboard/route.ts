import { getAccessToken } from "@auth0/nextjs-auth0";

import { NextResponse } from 'next/server'

export async function GET() {
  const accessToken = await getAccessToken();
  const userDetailsByIdUrl = `http://localhost:8080/api/service/health`;
  console.log(`fetching from ${userDetailsByIdUrl}`);
  const response = await fetch(userDetailsByIdUrl, {
    headers: {
      Authorization: `Bearer ${accessToken.accessToken}`
    }
  });
  return NextResponse.json({"response": response.statusText})
}