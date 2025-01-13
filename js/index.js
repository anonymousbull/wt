import PocketBase from 'pocketbase';

const pb = new PocketBase('http://134.209.23.64:8090');


const authData = await pb.collection("users")
    .authWithPassword("business@femi.market","Password1.")
    ;


console.log(authData)
// after the above you can also access the auth data from the authStore
// console.log(pb.authStore.isValid);
// console.log(pb.authStore.token);
// console.log(pb.authStore.record.id);
//
// // "logout" the last authenticated record
// pb.authStore.clear();