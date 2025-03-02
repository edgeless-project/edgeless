const session = require('express-session');
const Keycloak = require('keycloak-connect');
const db = require('../mongo/db_user');




const axios = require('axios');
require('dotenv').config();

async function getKeycloakToken(req, res) {
  const tokenUrl = process.env.KEYCLOAK_TOKEN_URL;
  const clientId = process.env.KEYCLOAK_CLIENT_ID;
  const clientSecret = process.env.KEYCLOAK_CLIENT_SECRET;
  // const username = process.env.KEYCLOAK_USERNAME;
  // const password = process.env.KEYCLOAK_PASSWORD;
  const username = req.body.username || process.env.KEYCLOAK_USERNAME; //read username from request body or use the default values
  const password = req.body.password || process.env.KEYCLOAK_PASSWORD;
  


  const params = new URLSearchParams();
  params.append('client_id', clientId);
  params.append('client_secret', clientSecret); 
  params.append('grant_type', 'password');
  params.append('username', username);
  params.append('password', password);

  console.log('Fetching Keycloak token...');

  try {
    const response = await axios.post(tokenUrl, params, {
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded'
      }
    });
    console.log('Keycloak token:', response.data.access_token);

    res.json({ token: response.data.access_token });

  } catch (error) {
    console.error('Error fetching Keycloak token:', error);
    throw error;
  }
}


async function validateToken(token) {
  // console.log(JSON.stringify(req.headers));
  // const token = req.headers.authorization.split(' ')[1];
  console.log('Validating token...');
  //send token to keycloak for validation and retrieve user information from token
  const keycloakUrl = process.env.KEYCLOAK_URL;
  const keycloakRealm = process.env.KEYCLOAK_REALM;
  const userInfoUrl = `${keycloakUrl}/realms/${keycloakRealm}/protocol/openid-connect/userinfo`;
  try {
    const response = await axios.get(userInfoUrl, {
      headers: {
          Authorization: `Bearer ${token}`,
      },
  });
  // console.log('User Info:', response.data);
  await storeUser(response.data);
  return response.data;

} catch (error) {
    console.error('Token verification failed:', error.response?.data || error.message);
    throw new Error('Invalid token');
  }
}

async function getKeycloakUser(req){
  const token = req.headers.authorization.split(' ')[1];
  try {

    const valid = await validateToken(token);
    if (valid){
      return valid; // This sends the user info back in the response
    }
  } catch (error) {
    console.error('Error validating token:', error);
    return { error: 'Invalid token' };
  }
}

const storeUser = async (userInfo) => {
  try {
      const { sub, preferred_username, email, given_name, family_name } = userInfo;

      // Use upsert to either update or insert a new user based on userId
      const result = await db.User.updateOne(
          { userId: sub }, // Find user by userId
          {
              $setOnInsert: {
                  username: preferred_username,
                  email,
                  first_name: given_name,
                  last_name: family_name,
                  registered_at: new Date(),
              },
          },
          { upsert: true } // Ensure the operation inserts if no document is found
      );

      if (result.upsertedCount > 0) {
          console.log('New user registered:', result.upsertedId);
      } else {
          // console.log('User Found and Updated!');
      }
  } catch (error) {
      console.error('Error storing user info:', error);
  }
};
module.exports = {
  getKeycloakToken,
  validateToken,
  getKeycloakUser
};